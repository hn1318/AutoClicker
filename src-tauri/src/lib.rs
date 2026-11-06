mod clicker;
mod commands;
mod cursor;
mod error;
mod hotkey;
mod state;
mod storage;
mod tray;
mod updater;
mod window_bind;

use tauri::Manager;

use state::{AppState, Settings};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Only release builds take the single-instance lock. In dev it would fight
    // with an installed copy sitting in the tray: the plugin makes the second
    // process exit immediately, so `tauri dev` died with a bare `-1`.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        // Relaunching from the Start menu must surface a hidden window.
        tray::show_main(app);
    }));

    builder
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::toggle_pause,
            commands::is_running,
            commands::is_recording,
            commands::start_recording,
            commands::stop_recording,
            commands::get_stats,
            commands::get_cursor_position,
            commands::set_cursor_tracking,
            commands::get_scale_at,
            commands::get_foreground_info,
            commands::get_app_info,
            commands::open_data_dir,
            commands::get_data_dir,
            commands::pick_data_dir,
            commands::set_data_dir,
            commands::reset_data_dir,
            commands::resolve_close,
            commands::export_settings,
            commands::import_settings,
            commands::get_logs,
            commands::clear_logs,
            updater::check_update,
            updater::download_update,
            updater::install_update,
        ])
        .setup(|app| {
            let handle = app.handle();
            let state = app.state::<AppState>();

            // Restore persisted settings before the first paint reads them.
            if let Some(mut settings) = storage::load(handle) {
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
            if let Err(error) = hotkey::apply(
                handle,
                &settings.toggle_hotkey,
                &settings.abort_hotkey,
                &settings.pause_hotkey,
            ) {
                eprintln!("[autoclicker] hotkey registration failed: {error}");
            }

            // The overlay must be built on the GUI thread; it stays hidden until
            // the UI asks for tracking.
            if let Err(error) = cursor::init(handle) {
                eprintln!("[autoclicker] cursor overlay unavailable: {error}");
            }

            // Tray + close policy. Without a tray the "keep running" branch would
            // leave the app invisible and unreachable.
            if let Err(error) = tray::init(handle, &settings.language) {
                eprintln!("[autoclicker] tray icon unavailable: {error}");
            }

            if let Some(window) = app.get_webview_window("main") {
                let handle = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // The policy decides; the window must not close on its own.
                        api.prevent_close();
                        tray::handle_close(&handle);
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
