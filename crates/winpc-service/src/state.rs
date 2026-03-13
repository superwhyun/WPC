use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;
use winpc_core::{AppConfig, DeviceStatus, Error, Result, UnlockExpiryAction};

#[derive(Clone)]
pub struct SharedState {
    inner: Arc<AppState>,
}

pub struct AppState {
    config_path: PathBuf,
    config: RwLock<AppConfig>,
    sessions: Mutex<HashMap<String, DateTime<Utc>>>,
    protected_user_logged_in: RwLock<bool>,
    last_spawn_attempt: Mutex<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingExpiryAction {
    pub action: UnlockExpiryAction,
    pub warn_only: bool,
    pub protected_user_sid: Option<String>,
}

impl SharedState {
    pub async fn load(path: PathBuf) -> Result<Self> {
        let config = AppConfig::load(&path)?;
        Ok(Self {
            inner: Arc::new(AppState {
                config_path: path,
                config: RwLock::new(config),
                sessions: Mutex::new(HashMap::new()),
                protected_user_logged_in: RwLock::new(false),
                last_spawn_attempt: Mutex::new(None),
            }),
        })
    }

    pub fn config_path(&self) -> &Path {
        &self.inner.config_path
    }

    pub async fn current_config(&self) -> AppConfig {
        self.inner.config.read().await.clone()
    }

    pub async fn replace_config(&self, config: AppConfig) -> Result<()> {
        self.persist_config(config).await
    }

    pub async fn verify_pin(&self, pin: &str) -> Result<()> {
        self.inner.config.read().await.verify_pin(pin)
    }

    pub async fn record_session(&self, token: String, expires_at_utc: DateTime<Utc>) {
        self.inner
            .sessions
            .lock()
            .await
            .insert(token, expires_at_utc);
    }

    pub async fn require_session(&self, token: &str) -> Result<()> {
        let mut sessions = self.inner.sessions.lock().await;
        let now = Utc::now();
        sessions.retain(|_, expires_at| *expires_at > now);
        match sessions.get(token) {
            Some(expires_at) if *expires_at > now => Ok(()),
            _ => Err(Error::InvalidSessionToken),
        }
    }

    pub async fn device_status(&self) -> DeviceStatus {
        let now = Utc::now();
        let config = self.current_config().await;
        config.status(now, *self.inner.protected_user_logged_in.read().await)
    }

    pub async fn clear_saved_agent_heartbeat(&self) -> Result<bool> {
        let mut config = self.current_config().await;
        let had_heartbeat = config.last_agent_heartbeat_utc.is_some();
        if had_heartbeat {
            config.last_agent_heartbeat_utc = None;
            self.persist_config(config).await?;
        }
        Ok(had_heartbeat)
    }

    pub async fn unlock(
        &self,
        duration_minutes: u16,
        expiry_action: Option<UnlockExpiryAction>,
    ) -> Result<DeviceStatus> {
        self.mutate_config(|config, now| config.unlock_until(duration_minutes, now, expiry_action))
            .await
    }

    pub async fn extend(
        &self,
        duration_minutes: u16,
        expiry_action: Option<UnlockExpiryAction>,
    ) -> Result<DeviceStatus> {
        self.mutate_config(|config, now| config.extend_unlock(duration_minutes, now, expiry_action))
            .await
    }

    pub async fn set_expiry_action(
        &self,
        expiry_action: UnlockExpiryAction,
    ) -> Result<DeviceStatus> {
        self.mutate_config(|config, _| {
            config.set_unlock_expiry_action(expiry_action);
            Ok(())
        })
        .await
    }

    pub async fn lock(&self) -> Result<DeviceStatus> {
        self.mutate_config(|config, _| {
            config.lock();
            Ok(())
        })
        .await
    }

    pub async fn record_heartbeat(&self) -> Result<DeviceStatus> {
        self.mutate_config(|config, now| {
            config.record_heartbeat(now);
            Ok(())
        })
        .await
    }

    pub async fn local_unlock(
        &self,
        pin: &str,
        duration_minutes: u16,
        expiry_action: Option<UnlockExpiryAction>,
    ) -> Result<DeviceStatus> {
        self.verify_pin(pin).await?;
        self.unlock(duration_minutes, expiry_action).await
    }

    pub async fn set_protected_user_logged_in(&self, value: bool) {
        *self.inner.protected_user_logged_in.write().await = value;
    }

    pub async fn should_retry_agent_spawn(&self, now: DateTime<Utc>) -> bool {
        let mut guard = self.inner.last_spawn_attempt.lock().await;
        let can_retry = guard
            .map(|last_attempt| now - last_attempt >= Duration::seconds(15))
            .unwrap_or(true);
        if can_retry {
            *guard = Some(now);
        }
        can_retry
    }

    pub async fn take_pending_expiry_action(&self) -> Result<Option<PendingExpiryAction>> {
        let now = Utc::now();
        let mut config = self.inner.config.write().await.clone();
        let Some(action) = config.take_expired_unlock_action(now) else {
            return Ok(None);
        };

        let pending = PendingExpiryAction {
            action,
            warn_only: config.warn_only,
            protected_user_sid: config.protected_user_sid.clone(),
        };
        self.persist_config(config).await?;
        Ok(Some(pending))
    }

    async fn mutate_config<F>(&self, f: F) -> Result<DeviceStatus>
    where
        F: FnOnce(&mut AppConfig, DateTime<Utc>) -> Result<()>,
    {
        let now = Utc::now();
        let mut config = self.inner.config.write().await.clone();
        f(&mut config, now)?;
        let status = config.status(now, *self.inner.protected_user_logged_in.read().await);
        self.persist_config(config).await?;
        Ok(status)
    }

    async fn persist_config(&self, config: AppConfig) -> Result<()> {
        config.save(&self.inner.config_path)?;
        *self.inner.config.write().await = config;
        Ok(())
    }

    pub async fn mark_agent_unhealthy_if_needed(&self) -> DeviceStatus {
        let now = Utc::now();
        let config = self.current_config().await;
        let status = config.status(now, *self.inner.protected_user_logged_in.read().await);
        if !status.agent_healthy && status.protected_user_logged_in {
            warn!("agent heartbeat is stale");
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use winpc_core::AppConfig;

    use super::SharedState;

    #[tokio::test]
    async fn clears_saved_agent_heartbeat() {
        let tempdir = tempfile::tempdir().unwrap();
        let state = SharedState::load(tempdir.path().join("config.json"))
            .await
            .unwrap();

        let mut config = AppConfig::default();
        config.record_heartbeat(Utc::now());
        state.replace_config(config).await.unwrap();

        assert!(state.clear_saved_agent_heartbeat().await.unwrap());
        assert!(state
            .current_config()
            .await
            .last_agent_heartbeat_utc
            .is_none());
        assert!(!state.clear_saved_agent_heartbeat().await.unwrap());
    }
}
