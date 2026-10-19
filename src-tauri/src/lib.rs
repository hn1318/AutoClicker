mod clicker;
mod commands;
mod cursor;
mod error;
mod hotkey;
mod state;
mod window_bind;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

use state::{AppState, Settings};

const STORE_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "settings";

fn load_settings(app: &tauri::AppHandle) -> Option<Settings> {
    let store = app.store(STORE_FILE).ok()?;
    let value = store.get(SETTINGS_KEY)?;
    serde_json::from_value::<Settings>(value).ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: it owns the second-launch handshake.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::new(Settings::default()))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_hotkeys,
            commands::start_clicking,
            commands::stop_clicking,
            commands::is_running,
            commands::get_cursor_position,
            commands::set_cursor_tracking,
            commands::get_scale_at,
            commands::open_url,
            commands::get_foreground_info,
            commands::get_app_info,
            commands::open_data_dir,
            commands::export_settings,
            commands::import_settings,
            commands::get_logs,
            commands::clear_logs,
        ])
        .setup(|app| {
            let handle = app.handle();
            let state = app.state::<AppState>();

            // Restore persisted settings before the first paint reads them.
            if let Some(mut settings) = load_settings(handle) {
                settings.normalize();
                if let Ok(mut guard) = state.settings.write() {
                    *guard = settings;
                }
            }

            let settings = state
                .settings
                .read()
                .map(|guard| guard.clone())
                .unwrap_or_default();

            // A conflicting hotkey must not abort startup — the UI reports it.
            if let Err(error) = hotkey::apply(handle, &settings.toggle_hotkey, &settings.abort_hotkey)
            {
                eprintln!("[autoclicker] hotkey registration failed: {error}");
            }

            // The overlay must be built on the GUI thread; it stays hidden until
            // the UI asks for tracking.
            if let Err(error) = cursor::init(handle) {
                eprintln!("[autoclicker] cursor overlay unavailable: {error}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
