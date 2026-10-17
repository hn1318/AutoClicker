use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{json, Value};
use thiserror::Error;

/// Every error crossing the IPC boundary is reduced to a stable machine code
/// plus optional parameters. The UI translates the code, so switching language
/// never leaves a stale message behind.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("already running")]
    AlreadyRunning,
    #[error("not running")]
    NotRunning,
    #[error("hotkey conflict: {0}")]
    HotkeyConflict(String),
    #[error("invalid hotkey: {0}")]
    HotkeyInvalid(String),
    #[error("input blocked")]
    InputBlocked,
    #[error("settings file is not valid JSON settings")]
    SettingsFileInvalid,
    #[error("platform error: {0}")]
    Platform(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalidInput",
            Self::AlreadyRunning => "alreadyRunning",
            Self::NotRunning => "notRunning",
            Self::HotkeyConflict(_) => "hotkeyConflict",
            Self::HotkeyInvalid(_) => "hotkeyInvalid",
            Self::InputBlocked => "inputBlocked",
            Self::SettingsFileInvalid => "settingsFileInvalid",
            Self::Platform(_) => "platform",
            Self::Internal(_) => "internal",
        }
    }

    pub fn params(&self) -> Value {
        match self {
            Self::InvalidInput(field) => json!({ "field": field }),
            Self::HotkeyConflict(accel) | Self::HotkeyInvalid(accel) => json!({ "accel": accel }),
            Self::Platform(detail) | Self::Internal(detail) => json!({ "detail": detail }),
            _ => Value::Null,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("params", &self.params())?;
        state.end()
    }
}
