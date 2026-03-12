use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceMode {
    Locked,
    Unlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub mode: DeviceMode,
    pub warn_only: bool,
    pub unlock_expires_at_utc: Option<DateTime<Utc>>,
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
pub struct LockCommandRequest {
    pub duration_minutes: u16,
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
    LocalUnlock { pin: String, duration_minutes: u16 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcResponse {
    State(DeviceStatus),
    Ack(DeviceStatus),
    Error { message: String },
}
