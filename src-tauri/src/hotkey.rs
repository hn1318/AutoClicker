//! Global hotkey registration with atomic rollback on conflict.
//!
//! Registration always happens in Rust: hotkeys keep working while the window
//! is hidden to the tray, and a conflicting accelerator is reported as a
//! structured error instead of silently doing nothing.

use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands;
use crate::error::AppError;
use crate::state::{AppState, HotkeyBindings};

fn register_all(app: &AppHandle, bindings: &HotkeyBindings) -> Result<(), AppError> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    manager
        .on_shortcut(bindings.toggle.as_str(), |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                commands::toggle_from_hotkey(app);
            }
        })
        .map_err(|_| AppError::HotkeyConflict(bindings.toggle.clone()))?;

    manager
        .on_shortcut(bindings.abort.as_str(), |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                commands::abort_from_hotkey(app);
            }
        })
        .map_err(|_| AppError::HotkeyConflict(bindings.abort.clone()))?;

    Ok(())
}

/// Register `toggle` / `abort`. On failure the previous bindings are restored.
pub fn apply(app: &AppHandle, toggle: &str, abort: &str) -> Result<HotkeyBindings, AppError> {
    let target = HotkeyBindings {
        toggle: toggle.trim().to_string(),
        abort: abort.trim().to_string(),
    };

    if target.toggle.is_empty() || target.abort.is_empty() {
        return Err(AppError::HotkeyInvalid(target.toggle.clone()));
    }
    if target.toggle.eq_ignore_ascii_case(&target.abort) {
        return Err(AppError::HotkeyInvalid(target.toggle.clone()));
    }

    let state = app.state::<AppState>();
    let previous = state
        .hotkeys
        .lock()
        .map_err(|_| AppError::Internal("hotkey state poisoned".into()))?
        .clone();

    if previous == target {
        return Ok(target);
    }

    match register_all(app, &target) {
        Ok(()) => {
            if let Ok(mut guard) = state.hotkeys.lock() {
                *guard = target.clone();
            }
            Ok(target)
        }
        Err(error) => {
            // Best effort: restore what the user had before this attempt.
            let _ = register_all(app, &previous);
            Err(error)
        }
    }
}

