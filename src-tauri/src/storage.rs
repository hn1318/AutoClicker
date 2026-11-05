//! Settings persistence with a user-relocatable data directory.
//!
//! Two files are involved:
//! - `location.json` — fixed, lives in the platform data dir, stores the custom
//!   directory (or nothing for the default location). Without a fixed pointer a
//!   relocated profile could never be found again.
//! - `settings.json` — the profile, in the directory `location.json` points at.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::state::Settings;

const LOCATION_FILE: &str = "location.json";
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Location {
    data_dir: String,
}

/// Platform default: `%APPDATA%\<identifier>` on Windows.
pub fn default_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn location_path(app: &AppHandle) -> PathBuf {
    default_dir(app).join(LOCATION_FILE)
}

fn read_location(app: &AppHandle) -> Location {
    std::fs::read_to_string(location_path(app))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Directory currently holding the profile.
pub fn current_dir(app: &AppHandle) -> PathBuf {
    let custom = read_location(app).data_dir;
    let trimmed = custom.trim();
    if trimmed.is_empty() {
        default_dir(app)
    } else {
        PathBuf::from(trimmed)
    }
}

pub fn settings_path(app: &AppHandle) -> PathBuf {
    current_dir(app).join(SETTINGS_FILE)
}

/// Load the profile, tolerating both the current flat format and the legacy
/// `{ "settings": { ... } }` wrapper written by older builds.
pub fn load(app: &AppHandle) -> Option<Settings> {
    let raw = std::fs::read_to_string(settings_path(app)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let payload = value.get("settings").cloned().unwrap_or(value);
    serde_json::from_value::<Settings>(payload).ok()
}

/// Write the profile atomically — a crash mid-write must not eat the settings.
pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), AppError> {
    let dir = current_dir(app);
    std::fs::create_dir_all(&dir).map_err(|error| AppError::Internal(error.to_string()))?;

    let path = dir.join(SETTINGS_FILE);
    let temp = dir.join(format!("{SETTINGS_FILE}.tmp"));
    let json = serde_json::to_string_pretty(settings)
        .map_err(|error| AppError::Internal(error.to_string()))?;

    std::fs::write(&temp, json).map_err(|error| AppError::Internal(error.to_string()))?;
    std::fs::rename(&temp, &path).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(())
}

/// Point the app at `target`, moving the existing profile along.
/// Returns the directory that is now in use.
pub fn set_dir(app: &AppHandle, target: &str) -> Result<PathBuf, AppError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("dataDir"));
    }

    let target = PathBuf::from(trimmed);
    if !target.is_absolute() {
        return Err(AppError::InvalidInput("dataDir"));
    }
    std::fs::create_dir_all(&target).map_err(|error| AppError::Internal(error.to_string()))?;

    // Move the profile over, but never clobber one that is already there.
    let source = settings_path(app);
    let destination = target.join(SETTINGS_FILE);
    if source.exists() && !destination.exists() {
        std::fs::copy(&source, &destination)
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }

    write_location(app, &target)?;
    Ok(target)
}

/// Forget the custom directory and go back to the platform default.
pub fn reset_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let default = default_dir(app);
    std::fs::create_dir_all(&default).map_err(|error| AppError::Internal(error.to_string()))?;

    let source = settings_path(app);
    let destination = default.join(SETTINGS_FILE);
    if source.exists() && !destination.exists() {
        std::fs::copy(&source, &destination)
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }

    let _ = std::fs::remove_file(location_path(app));
    Ok(default)
}

fn write_location(app: &AppHandle, dir: &std::path::Path) -> Result<(), AppError> {
    let base = default_dir(app);
    std::fs::create_dir_all(&base).map_err(|error| AppError::Internal(error.to_string()))?;
    let location = Location {
        data_dir: dir.to_string_lossy().to_string(),
    };
    let json = serde_json::to_string_pretty(&location)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    std::fs::write(location_path(app), json).map_err(|error| AppError::Internal(error.to_string()))
}
