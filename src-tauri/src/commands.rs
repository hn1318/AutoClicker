//! All IPC surface. Errors are typed (`AppError`) so the frontend can render
//! localized messages instead of raw English strings.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::clicker::{SessionConfig, engine, inject};
use crate::error::AppError;
use crate::hotkey;
use crate::state::{AppState, EngineHandle, LogEntry, Settings};

fn state_of(app: &AppHandle) -> State<'_, AppState> {
    app.state::<AppState>()
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    state_of(&app)
        .settings
        .read()
        .map(|settings| settings.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, mut settings: Settings) -> Result<Settings, AppError> {
    settings.normalize();
    let state = state_of(&app);
    let mut guard = state
        .settings
        .write()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
    *guard = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn set_hotkeys(app: AppHandle, toggle: String, abort: String) -> Result<(), AppError> {
    hotkey::apply(&app, &toggle, &abort)?;
    let state = state_of(&app);
    let mut settings = state
        .settings
        .write()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
    settings.toggle_hotkey = toggle.trim().to_string();
    settings.abort_hotkey = abort.trim().to_string();
    Ok(())
}

pub fn is_engine_running(app: &AppHandle) -> bool {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(guard) = state.engine.lock() {
            if let Some(handle) = guard.as_ref() {
                return !handle.thread.is_finished();
            }
        }
    }
    false
}

#[tauri::command]
pub fn is_running(app: AppHandle) -> bool {
    is_engine_running(&app)
}

pub fn start_impl(app: &AppHandle) -> Result<(), AppError> {
    let state = state_of(app);
    let mut guard = state
        .engine
        .lock()
        .map_err(|_| AppError::Internal("engine lock poisoned".into()))?;

    if let Some(handle) = guard.as_ref() {
        if !handle.thread.is_finished() {
            return Err(AppError::AlreadyRunning);
        }
    }
    *guard = None;

    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?
        .clone();
    let config = SessionConfig::from_settings(&settings)?;

    let cancel = Arc::new(AtomicBool::new(false));
    let thread = engine::spawn(app.clone(), config, state.stats.clone(), cancel.clone())?;
    *guard = Some(EngineHandle { cancel, thread });

    state.log(
        "startRequested",
        json!({ "intervalMs": settings.interval_ms }),
    );
    Ok(())
}

pub fn stop_impl(app: &AppHandle) -> Result<(), AppError> {
    let state = state_of(app);
    let handle = {
        let mut guard = state
            .engine
            .lock()
            .map_err(|_| AppError::Internal("engine lock poisoned".into()))?;
        guard.take()
    };

    match handle {
        Some(handle) => {
            handle.cancel.store(true, Ordering::SeqCst);
            // Joining guarantees no click fires after this command returns.
            let _ = handle.thread.join();
            Ok(())
        }
        None => Err(AppError::NotRunning),
    }
}

#[tauri::command]
pub fn start_clicking(app: AppHandle) -> Result<(), AppError> {
    start_impl(&app)
}

#[tauri::command]
pub fn stop_clicking(app: AppHandle) -> Result<(), AppError> {
    stop_impl(&app)
}

/// Called from the global hotkey handler (works while hidden to tray).
pub fn toggle_from_hotkey(app: &AppHandle) {
    let outcome = if is_engine_running(app) {
        stop_impl(app)
    } else {
        start_impl(app)
    };
    if let Err(error) = outcome {
        emit_error(app, &error);
    }
}

pub fn abort_from_hotkey(app: &AppHandle) {
    if let Err(error) = stop_impl(app) {
        if !matches!(error, AppError::NotRunning) {
            emit_error(app, &error);
        }
    }
}

fn emit_error(app: &AppHandle, error: &AppError) {
    let _ = app.emit(
        "runtime:event",
        engine::RunEvent {
            event: "error".into(),
            params: json!({ "code": error.code(), "params": error.params() }),
        },
    );
    if let Some(state) = app.try_state::<AppState>() {
        state.log(
            "error",
            json!({ "code": error.code(), "params": error.params() }),
        );
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorInfo {
    pub physical_x: i32,
    pub physical_y: i32,
    pub logical_x: f64,
    pub logical_y: f64,
    pub scale: f64,
}

#[tauri::command]
pub fn get_scale_at(x: i32, y: i32) -> f64 {
    inject::scale_at(x, y)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundInfoPayload {
    pub title: String,
    pub process: String,
}

#[tauri::command]
pub fn get_foreground_info() -> Option<ForegroundInfoPayload> {
    crate::window_bind::foreground_info().map(|info| ForegroundInfoPayload {
        title: info.title,
        process: info.process,
    })
}

#[tauri::command]
pub fn get_cursor_position() -> CursorInfo {
    let (x, y) = inject::cursor_position();
    let scale = inject::scale_at(x, y);
    CursorInfo {
        physical_x: x,
        physical_y: y,
        logical_x: (x as f64 / scale).round(),
        logical_y: (y as f64 / scale).round(),
        scale,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: app
            .path()
            .app_data_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    std::fs::create_dir_all(&dir).map_err(|error| AppError::Internal(error.to_string()))?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(())
}

/// Suggested file name in the save dialog.
const SETTINGS_FILE_NAME: &str = "autoclicker-settings.json";

/// Export the current settings to a user-chosen `.json` file.
///
/// Returns `None` when the user cancels. The dialog is native and owned by Rust
/// so it works identically while the window is hidden to the tray.
#[tauri::command]
pub async fn export_settings(
    app: AppHandle,
    settings: Settings,
    title: String,
) -> Result<Option<String>, AppError> {
    let mut settings = settings;
    settings.normalize();
    let payload = serde_json::to_string_pretty(&settings)
        .map_err(|error| AppError::Internal(error.to_string()))?;

    // The blocking dialog parks the calling thread on a channel, so it must not
    // run on the main thread or on the async runtime's worker.
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title(title)
            .set_file_name(SETTINGS_FILE_NAME)
            .add_filter("JSON", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some(path) = picked.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };

    std::fs::write(&path, payload).map_err(|error| AppError::Internal(error.to_string()))?;
    let path = path.to_string_lossy().to_string();
    state_of(&app).log("settingsExported", json!({ "path": path }));
    Ok(Some(path))
}

/// Load settings from a user-chosen `.json` file and make them authoritative.
///
/// Returns `None` when the user cancels. Hotkeys are re-registered by the UI
/// afterwards, since only the frontend owns the "restore previous binding"
/// workflow.
#[tauri::command]
pub async fn import_settings(
    app: AppHandle,
    title: String,
) -> Result<Option<Settings>, AppError> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title(title)
            .add_filter("JSON", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some(path) = picked.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };

    let raw = std::fs::read_to_string(&path).map_err(|error| AppError::Internal(error.to_string()))?;
    // `Settings` is `#[serde(default)]`, so a partial file stays usable.
    let mut settings: Settings =
        serde_json::from_str(&raw).map_err(|_| AppError::SettingsFileInvalid)?;
    settings.normalize();

    let state = state_of(&app);
    {
        let mut guard = state
            .settings
            .write()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        *guard = settings.clone();
    }
    state.log(
        "settingsImported",
        json!({ "path": path.to_string_lossy() }),
    );
    Ok(Some(settings))
}

#[tauri::command]
pub fn get_logs(app: AppHandle) -> Vec<LogEntry> {
    state_of(&app)
        .logs
        .lock()
        .map(|logs| logs.snapshot())
        .unwrap_or_default()
}

#[tauri::command]
pub fn clear_logs(app: AppHandle) {
    if let Ok(mut logs) = state_of(&app).logs.lock() {
        logs.clear();
    }
}
