use std::{
    fs,
    path::{Path, PathBuf},
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

use crate::{
    model::{DeviceMode, DeviceStatus},
    security::{seal_bytes, unseal_bytes},
    Error, Result,
};

const HEARTBEAT_TIMEOUT_SECS: i64 = 20;
const MIN_DURATION_MINUTES: u16 = 1;
const MAX_DURATION_MINUTES: u16 = 480;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub protected_user_sid: Option<String>,
    pub pin_hash: Option<String>,
    #[serde(default)]
    pub warn_only: bool,
    pub unlock_expires_at_utc: Option<DateTime<Utc>>,
    pub last_agent_heartbeat_utc: Option<DateTime<Utc>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            protected_user_sid: None,
            pin_hash: None,
            warn_only: false,
            unlock_expires_at_utc: None,
            last_agent_heartbeat_utc: None,
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => Ok(serde_json::from_str(&raw)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn set_pin(&mut self, pin: &str) -> Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|_| Error::SecretSeal)?
            .to_string();
        let sealed = seal_bytes(hash.as_bytes())?;
        self.pin_hash = Some(STANDARD.encode(sealed));
        Ok(())
    }

    pub fn verify_pin(&self, pin: &str) -> Result<()> {
        let encoded = self
            .pin_hash
            .as_ref()
            .ok_or(Error::ConfigIncomplete("pin_hash"))?;
        let sealed = STANDARD.decode(encoded).map_err(|_| Error::SecretUnseal)?;
        let plaintext = unseal_bytes(&sealed)?;
        let hash = std::str::from_utf8(&plaintext).map_err(|_| Error::SecretUnseal)?;
        let parsed = PasswordHash::new(hash).map_err(|_| Error::SecretUnseal)?;
        Argon2::default()
            .verify_password(pin.as_bytes(), &parsed)
            .map_err(|_| Error::InvalidPin)
    }

    pub fn validate_duration_minutes(duration_minutes: u16) -> Result<()> {
        if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&duration_minutes) {
            return Err(Error::InvalidDuration);
        }
        Ok(())
    }

    pub fn lock(&mut self) {
        self.unlock_expires_at_utc = None;
    }

    pub fn unlock_until(&mut self, duration_minutes: u16, now: DateTime<Utc>) -> Result<()> {
        Self::validate_duration_minutes(duration_minutes)?;
        self.unlock_expires_at_utc = Some(now + Duration::minutes(duration_minutes as i64));
        Ok(())
    }

    pub fn extend_unlock(&mut self, duration_minutes: u16, now: DateTime<Utc>) -> Result<()> {
        Self::validate_duration_minutes(duration_minutes)?;
        let baseline = self.unlock_expires_at_utc.unwrap_or(now).max(now);
        self.unlock_expires_at_utc = Some(baseline + Duration::minutes(duration_minutes as i64));
        Ok(())
    }

    pub fn record_heartbeat(&mut self, now: DateTime<Utc>) {
        self.last_agent_heartbeat_utc = Some(now);
    }

    pub fn effective_mode(&self, now: DateTime<Utc>) -> DeviceMode {
        match self.unlock_expires_at_utc {
            Some(expires_at) if expires_at > now => DeviceMode::Unlocked,
            _ => DeviceMode::Locked,
        }
    }

    pub fn remaining_minutes(&self, now: DateTime<Utc>) -> u64 {
        let Some(expires_at) = self.unlock_expires_at_utc else {
            return 0;
        };
        if expires_at <= now {
            return 0;
        }

        let remaining_secs = (expires_at - now).num_seconds();
        ((remaining_secs + 59) / 60) as u64
    }

    pub fn agent_healthy(&self, now: DateTime<Utc>) -> bool {
        self.last_agent_heartbeat_utc
            .map(|heartbeat| (now - heartbeat).num_seconds() <= HEARTBEAT_TIMEOUT_SECS)
            .unwrap_or(false)
    }

    pub fn status(&self, now: DateTime<Utc>, protected_user_logged_in: bool) -> DeviceStatus {
        DeviceStatus {
            mode: self.effective_mode(now),
            warn_only: self.warn_only,
            unlock_expires_at_utc: self
                .unlock_expires_at_utc
                .filter(|expires_at| *expires_at > now),
            remaining_minutes: self.remaining_minutes(now),
            agent_healthy: self.agent_healthy(now),
            protected_user_logged_in,
            last_seen_at_utc: self.last_agent_heartbeat_utc,
        }
    }
}

pub fn default_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        PathBuf::from(base)
            .join("WinParentalControl")
            .join("config.json")
    }

    #[cfg(not(windows))]
    {
        std::env::temp_dir()
            .join("WinParentalControl")
            .join("config.json")
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::AppConfig;

    #[test]
    fn pin_roundtrip() {
        let mut config = AppConfig::default();
        config.set_pin("4321").unwrap();

        config.verify_pin("4321").unwrap();
        assert!(config.verify_pin("1111").is_err());
    }

    #[test]
    fn unlock_and_extend_work() {
        let now = Utc::now();
        let mut config = AppConfig::default();

        config.unlock_until(30, now).unwrap();
        assert_eq!(config.remaining_minutes(now), 30);

        config.extend_unlock(30, now).unwrap();
        assert_eq!(config.remaining_minutes(now), 60);
    }

    #[test]
    fn expired_unlock_becomes_locked() {
        let now = Utc::now();
        let config = AppConfig {
            warn_only: false,
            unlock_expires_at_utc: Some(now - Duration::minutes(1)),
            ..AppConfig::default()
        };

        assert_eq!(config.remaining_minutes(now), 0);
        assert_eq!(config.effective_mode(now), crate::model::DeviceMode::Locked);
    }
}
