use std::{io, num::ParseIntError};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("configuration is incomplete: {0}")]
    ConfigIncomplete(&'static str),
    #[error("failed to parse configuration: {0}")]
    ConfigParse(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("pin verification failed")]
    InvalidPin,
    #[error("duration must be between 1 and 480 minutes")]
    InvalidDuration,
    #[error("session token is missing or invalid")]
    InvalidSessionToken,
    #[error("failed to parse integer: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("sensitive data decryption failed")]
    SecretUnseal,
    #[error("sensitive data encryption failed")]
    SecretSeal,
    #[error("time handling failed")]
    Time,
}

pub type Result<T> = std::result::Result<T, Error>;
