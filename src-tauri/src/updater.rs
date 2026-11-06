//! In-app updates.
//!
//! The plugin verifies the minisign signature carried by the release manifest,
//! so a tampered endpoint cannot push arbitrary code. Download and install are
//! deliberately two commands: installing closes the app (`std::process::exit`
//! inside the Windows installer path), which must never happen mid-run without
//! the user asking for it.

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use crate::error::AppError;
use crate::state::{AppState, PendingUpdate};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: String,
    pub date: Option<String>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

fn describe(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        date: update.date.map(|date| date.to_string()),
    }
}

fn failed(error: impl std::fmt::Display) -> AppError {
    AppError::UpdateFailed(error.to_string())
}

/// A non-empty `source` replaces the endpoints from `tauri.conf.json`; it has
/// to serve the same manifest shape (`version` + `platforms[...]`).
fn build(app: &AppHandle, source: Option<&str>) -> Result<Updater, AppError> {
    let builder = app.updater_builder();
    let builder = match source.map(str::trim).filter(|value| !value.is_empty()) {
        Some(url) => {
            let parsed = tauri::Url::parse(url).map_err(|_| AppError::InvalidInput("updateUrl"))?;
            builder.endpoints(vec![parsed]).map_err(failed)?
        }
        None => builder,
    };
    builder.build().map_err(failed)
}

async fn lookup(app: &AppHandle, source: Option<&str>) -> Result<Option<Update>, AppError> {
    let updater = build(app, source)?;
    updater.check().await.map_err(failed)
}

#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    source: Option<String>,
) -> Result<Option<UpdateInfo>, AppError> {
    Ok(lookup(&app, source.as_deref()).await?.as_ref().map(describe))
}

/// Fetches and verifies the installer, but does not run it. The verified bytes
/// stay in memory until `install_update` is called.
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    source: Option<String>,
) -> Result<UpdateInfo, AppError> {
    let update = lookup(&app, source.as_deref())
        .await?
        .ok_or(AppError::UpdateNotFound)?;

    // The plugin reports each chunk separately, so the running total is ours.
    let progress = app.clone();
    let mut downloaded: u64 = 0;
    let bytes = update
        .download(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress.emit(
                    "update:progress",
                    UpdateProgress { downloaded, total },
                );
            },
            || {},
        )
        .await
        .map_err(failed)?;

    let info = describe(&update);
    let state = app.state::<AppState>();
    *state.pending_update.lock().map_err(|_| failed("update slot poisoned"))? =
        Some(PendingUpdate {
            version: info.version.clone(),
            update,
            bytes,
        });
    state.log("updateDownloaded", json!({ "version": info.version }));
    Ok(info)
}

/// Runs the cached installer and exits. The NSIS installer restarts the app on
/// the new version, which is the "click restart to finish" step in the UI.
#[tauri::command]
pub fn install_update(app: AppHandle) -> Result<(), AppError> {
    let pending = app
        .state::<AppState>()
        .pending_update
        .lock()
        .map_err(|_| failed("update slot poisoned"))?
        .take()
        .ok_or(AppError::UpdateNotReady)?;

    app.state::<AppState>()
        .log("updateInstalling", json!({ "version": pending.version }));
    pending.update.install(pending.bytes).map_err(failed)
}
