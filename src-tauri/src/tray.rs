//! Tray icon and close-button policy.
//!
//! The window's close request is always intercepted: depending on
//! `Settings::close_action` the app either asks the UI, keeps running in the
//! tray, or exits. A tray is mandatory for the "keep running" branch — otherwise
//! a hidden window could never be brought back.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

use crate::state::{AppState, CloseAction};

pub const TRAY_ID: &str = "main-tray";
pub const CLOSE_REQUEST_EVENT: &str = "app:close-requested";

fn is_chinese(language: &str) -> bool {
    match language {
        "zh-CN" => true,
        "en-US" => false,
        // "system": follow the Windows UI language.
        _ => unsafe { (GetUserDefaultUILanguage() & 0x03FF) == 0x0004 },
    }
}

fn build_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let chinese = is_chinese(language);
    let show = MenuItem::with_id(
        app,
        "show",
        if chinese { "显示主窗口" } else { "Show AutoClicker" },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if chinese { "退出" } else { "Quit" },
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show, &quit])
}

pub fn init(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let menu = build_menu(app, language)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("AutoClicker")
        .menu(&menu)
        // Left click restores the window; the menu stays on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the menu after the UI language changed.
pub fn refresh_menu(app: &AppHandle, language: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if let Ok(menu) = build_menu(app, language) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn hide_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Hide to the tray, but only if a tray actually exists to come back from.
pub fn keep_in_tray(app: &AppHandle) {
    if app.tray_by_id(TRAY_ID).is_some() {
        hide_to_tray(app);
    } else {
        app.exit(0);
    }
}

/// Apply the configured close behaviour. The caller has already prevented the
/// default close, so every branch must finish the job.
pub fn handle_close(app: &AppHandle) {
    let action = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .settings
                .read()
                .map(|settings| settings.close_action)
                .ok()
        })
        .unwrap_or(CloseAction::Quit);

    match action {
        CloseAction::Tray => keep_in_tray(app),
        CloseAction::Quit => app.exit(0),
        CloseAction::Ask => {
            // Make sure the dialog is actually in front of the user.
            show_main(app);
            let _ = app.emit_to("main", CLOSE_REQUEST_EVENT, ());
        }
    }
}
