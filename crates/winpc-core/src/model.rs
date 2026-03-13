use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceMode {
    Locked,
    Unlocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnlockExpiryAction {
    #[serde(alias = "agent_lock")]
    AppLock,
    WindowsLock,
    Shutdown,
}

impl Default for UnlockExpiryAction {
    fn default() -> Self {
        Self::AppLock
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub mode: DeviceMode,
    pub warn_only: bool,
    pub unlock_expires_at_utc: Option<DateTime<Utc>>,
    pub unlock_expiry_action: Option<UnlockExpiryAction>,
    pub remaining_minutes: u64,
    pub agent_healthy: bool,
    pub protected_user_logged_in: bool,
    pub last_seen_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthPinRequest {
    pub pin: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthPinResponse {
    pub token: String,
    pub expires_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePinRequest {
    pub current_pin: String,
    pub new_pin: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockCommandRequest {
    pub duration_minutes: i16,
    #[serde(default)]
    pub expiry_action: Option<UnlockExpiryAction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockActionResponse {
    pub status: DeviceStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcRequest {
    GetState,
    Heartbeat,
    LocalUnlock {
        pin: String,
        duration_minutes: i16,
        #[serde(default)]
        expiry_action: Option<UnlockExpiryAction>,
    },
    LocalExtend {
        pin: String,
        duration_minutes: i16,
        #[serde(default)]
        expiry_action: Option<UnlockExpiryAction>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcResponse {
    State(DeviceStatus),
    Ack(DeviceStatus),
    Error { message: String },
}

pub const AGENT_COMMAND_PIPE_NAME: &str = r"\\.\pipe\WinParentalControlAgentCommands";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentCommandRequest {
    CaptureSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentCommandResponse {
    Snapshot { png_base64: String },
    Error { message: String },
}
