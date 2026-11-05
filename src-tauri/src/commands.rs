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
use crate::state::{
    AppState, ClickType, CloseAction, EngineHandle, LogEntry, RecordedClick, SequenceStep, Settings,
};

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

    let previous_language = {
        let state = state_of(&app);
        let mut guard = state
            .settings
            .write()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        let previous = guard.language.clone();
        *guard = settings.clone();
        previous
    };

    // The profile must reach the disk here: there is no frontend store anymore.
    crate::storage::save(&app, &settings)?;

    if previous_language != settings.language {
        sync_tray_language(&app, &settings.language);
    }
    Ok(settings)
}

#[tauri::command]
pub fn set_hotkeys(
    app: AppHandle,
    toggle: String,
    abort: String,
    pause: String,
) -> Result<(), AppError> {
    hotkey::apply(&app, &toggle, &abort, &pause)?;
    let state = state_of(&app);
    let mut settings = state
        .settings
        .write()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
    settings.toggle_hotkey = toggle.trim().to_string();
    settings.abort_hotkey = abort.trim().to_string();
    settings.pause_hotkey = pause.trim().to_string();
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

    state.pause.store(false, Ordering::SeqCst);
    let cancel = Arc::new(AtomicBool::new(false));
    let thread = engine::spawn(
        app.clone(),
        config,
        state.stats.clone(),
        cancel.clone(),
        state.pause.clone(),
    )?;
    *guard = Some(EngineHandle { cancel, thread });

    state.log(
        "startRequested",
        json!({ "intervalMs": settings.interval_ms }),
    );
    Ok(())
}

pub fn stop_impl(app: &AppHandle) -> Result<(), AppError> {
    let state = state_of(app);
    state.pause.store(false, Ordering::SeqCst);
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

fn toggle_pause_impl(app: &AppHandle) -> Result<bool, AppError> {
    let state = state_of(app);
    if !is_engine_running(app) {
        return Err(AppError::NotRunning);
    }
    let paused = !state.pause.load(Ordering::SeqCst);
    state.pause.store(paused, Ordering::SeqCst);
    state.log("pauseToggled", json!({ "paused": paused }));
    Ok(paused)
}

#[tauri::command]
pub fn toggle_pause(app: AppHandle) -> Result<bool, AppError> {
    toggle_pause_impl(&app)
}

/// Hotkey entry point: no-op when nothing is running.
pub fn pause_from_hotkey(app: &AppHandle) {
    if let Err(error) = toggle_pause_impl(app) {
        if !matches!(error, AppError::NotRunning) {
            emit_error(app, &error);
        }
    }
}

/* ---------------- sequence recording ---------------- */

#[tauri::command]
pub fn is_recording(app: AppHandle) -> bool {
    crate::clicker::recorder::is_recording(&app)
}

#[tauri::command]
pub fn start_recording(app: AppHandle) -> Result<(), AppError> {
    // A running engine injects clicks the recorder must not pick up.
    if is_engine_running(&app) {
        stop_impl(&app)?;
    }
    crate::clicker::recorder::start(&app)?;
    state_of(&app).log("recordingStarted", json!({}));
    Ok(())
}

#[tauri::command]
pub fn stop_recording(app: AppHandle) -> Result<Vec<SequenceStep>, AppError> {
    let clicks = crate::clicker::recorder::stop(&app)?;
    let steps = clicks_to_steps(&clicks);
    state_of(&app).log(
        "recordingStopped",
        json!({ "clicks": clicks.len(), "steps": steps.len() }),
    );
    Ok(steps)
}

/// Recorded clicks become a one-shot sequence, with the gap after each click as
/// its delay.
fn clicks_to_steps(clicks: &[RecordedClick]) -> Vec<SequenceStep> {
    let mut steps = Vec::with_capacity(clicks.len());
    for (index, click) in clicks.iter().enumerate() {
        let delay = clicks
            .get(index + 1)
            .map(|next| next.ts_ms.saturating_sub(click.ts_ms))
            .unwrap_or(200)
            .clamp(10, 600_000);
        steps.push(SequenceStep {
            id: format!("rec-{}-{}", index, click.ts_ms),
            x: click.x,
            y: click.y,
            button: click.button,
            click_type: ClickType::Single,
            hold_ms: 40,
            delay_ms: delay,
            enabled: true,
        });
    }
    steps
}

/* ---------------- stats ---------------- */

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSnapshot {
    pub clicks: u64,
    pub elapsed_ms: u64,
    pub cps: f64,
    pub running: bool,
    pub paused: bool,
}

#[tauri::command]
pub fn get_stats(app: AppHandle) -> StatsSnapshot {
    let state = state_of(&app);
    let clicks = state.stats.clicks.load(Ordering::Relaxed);
    let started = state.stats.started_at_ms.load(Ordering::Relaxed);
    let elapsed_ms = if started > 0 {
        crate::state::unix_ms().saturating_sub(started)
    } else {
        0
    };

    StatsSnapshot {
        clicks,
        elapsed_ms,
        cps: if elapsed_ms > 0 {
            clicks as f64 * 1000.0 / elapsed_ms as f64
        } else {
            0.0
        },
        running: is_engine_running(&app),
        paused: state.pause.load(Ordering::SeqCst),
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

/// Show or hide the pointer readout overlay (see `cursor.rs`).
#[tauri::command]
pub fn set_cursor_tracking(app: AppHandle, active: bool) -> Result<(), AppError> {
    if active {
        crate::cursor::start(&app)
    } else {
        crate::cursor::stop(&app)
    }
}

/// Open an `http(s)` URL in the default browser (update downloads).
#[tauri::command]
pub fn open_url(url: String) -> Result<(), AppError> {
    let target = url.trim();
    if !(target.starts_with("https://") || target.starts_with("http://")) {
        return Err(AppError::InvalidInput("updateUrl"));
    }
    std::process::Command::new("explorer")
        .arg(target)
        .spawn()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(())
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
        data_dir: crate::storage::current_dir(&app)
            .to_string_lossy()
            .to_string(),
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), AppError> {
    let dir = crate::storage::current_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|error| AppError::Internal(error.to_string()))?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn get_data_dir(app: AppHandle) -> String {
    crate::storage::current_dir(&app)
        .to_string_lossy()
        .to_string()
}

/// Native folder picker for the "change data directory" flow.
#[tauri::command]
pub async fn pick_data_dir(app: AppHandle, title: String) -> Result<Option<String>, AppError> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title(title)
            .blocking_pick_folder()
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(picked
        .and_then(|folder| folder.into_path().ok())
        .map(|path| path.to_string_lossy().to_string()))
}

/// Switch to `dir`, carrying the current profile over. Returns the new path.
#[tauri::command]
pub fn set_data_dir(app: AppHandle, dir: String) -> Result<String, AppError> {
    let target = crate::storage::set_dir(&app, &dir)?;

    // Persist the in-memory profile into the new location right away.
    let settings = state_of(&app)
        .settings
        .read()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?
        .clone();
    crate::storage::save(&app, &settings)?;

    let path = target.to_string_lossy().to_string();
    state_of(&app).log("dataDirChanged", json!({ "path": path }));
    Ok(path)
}

/// Go back to the platform default directory.
#[tauri::command]
pub fn reset_data_dir(app: AppHandle) -> Result<String, AppError> {
    let target = crate::storage::reset_dir(&app)?;
    let settings = state_of(&app)
        .settings
        .read()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?
        .clone();
    crate::storage::save(&app, &settings)?;

    let path = target.to_string_lossy().to_string();
    state_of(&app).log("dataDirReset", json!({ "path": path }));
    Ok(path)
}

/// Answer to the "close to tray or quit?" dialog.
#[tauri::command]
pub fn resolve_close(app: AppHandle, action: String, remember: bool) -> Result<(), AppError> {
    let next = match action.as_str() {
        "tray" => CloseAction::Tray,
        "quit" => CloseAction::Quit,
        _ => CloseAction::Ask,
    };

    if remember && next != CloseAction::Ask {
        let settings = {
            let state = state_of(&app);
            let mut guard = state
                .settings
                .write()
                .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
            guard.close_action = next;
            guard.clone()
        };
        crate::storage::save(&app, &settings)?;
    }

    match next {
        CloseAction::Tray => crate::tray::keep_in_tray(&app),
        CloseAction::Quit => app.exit(0),
        CloseAction::Ask => {}
    }
    Ok(())
}

/// The tray menu follows the UI language, so a language change rebuilds it.
fn sync_tray_language(app: &AppHandle, language: &str) {
    crate::tray::refresh_menu(app, language);
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
    let previous_language = {
        let mut guard = state
            .settings
            .write()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        let previous = guard.language.clone();
        *guard = settings.clone();
        previous
    };
    crate::storage::save(&app, &settings)?;
    if previous_language != settings.language {
        sync_tray_language(&app, &settings.language);
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
