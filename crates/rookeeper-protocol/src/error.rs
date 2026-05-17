use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum ErrorCode {
    Success = 0,
    PathNotFound = 1,
    PathAlreadyExists = 2,
    VersionConflict = 3,
    PermissionDenied = 4,
    InvalidPath = 5,
    SessionExpired = 6,
    LockBusy = 7,
    ResourceExhausted = 8,
    StorageCorruption = 9,
    TransportUnavailable = 10,
    UnsupportedVersion = 11,
    Internal = 255,
}

impl ErrorCode {
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code:?}: {message}")]
pub struct RookeeperError {
    pub code: ErrorCode,
    pub message: String,
}

impl RookeeperError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
