//! Live cursor readout.
//!
//! A dedicated always-on-top, click-through webview window rides next to the
//! pointer and shows the physical/logical coordinates plus the monitor scale.
//! The sampler runs on its own thread and only wakes the UI when the pointer
//! actually moved, so an idle desktop costs nothing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetSystemMetrics, GetWindowLongPtrW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SetWindowLongPtrW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use crate::clicker::inject;
use crate::error::AppError;
use crate::state::{AppState, CursorWatch};

pub const OVERLAY_LABEL: &str = "cursor-overlay";
pub const EVENT_NAME: &str = "cursor:position";

/// Overlay geometry in physical pixels. Kept in sync with `CursorOverlay.vue`.
const OVERLAY_W: i32 = 170;
const OVERLAY_H: i32 = 34;
/// Gap between the pointer and the overlay so the target point stays visible.
const OVERLAY_GAP: i32 = 20;
/// 50 Hz is smooth enough for a readout and stays cheap.
const TICK: Duration = Duration::from_millis(20);

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPayload {
    pub physical_x: i32,
    pub physical_y: i32,
    pub logical_x: i32,
    pub logical_y: i32,
    pub scale: f64,
}

/// Create the overlay window up front (main thread) and keep it hidden.
///
/// Windows must be created on the GUI thread, so this is called once from
/// `setup`; starting tracking then only flips visibility and spawns a sampler.
pub fn init(app: &AppHandle) -> Result<(), AppError> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("AutoClicker cursor")
    .inner_size(OVERLAY_W as f64, OVERLAY_H as f64)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .visible(false)
    .build()
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let _ = window.set_size(PhysicalSize::new(OVERLAY_W as u32, OVERLAY_H as u32));
    make_click_through(&window);
    Ok(())
}

/// Let every click fall through to whatever is underneath the overlay.
fn make_click_through(window: &WebviewWindow) {
    // Use tao's own flag: it coexists with the DWM-based window transparency.
    // Adding WS_EX_LAYERED by hand would fight the transparent webview.
    let _ = window.set_ignore_cursor_events(true);

    // Keep the overlay out of Alt+Tab and make sure it can never take focus.
    let Ok(raw) = window.hwnd() else { return };
    let hwnd = HWND(raw.0 as *mut core::ffi::c_void);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let extra = (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE).0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | extra);
    }
}

pub fn start(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let mut guard = state
        .cursor_watch
        .lock()
        .map_err(|_| AppError::Internal("cursor watch lock poisoned".into()))?;

    if let Some(handle) = guard.as_ref() {
        if !handle.thread.is_finished() {
            return Ok(());
        }
    }
    *guard = None;

    let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) else {
        return Err(AppError::Internal("cursor overlay window missing".into()));
    };
    // Park it before showing so the window never flashes at its default spot.
    let (x, y) = inject::cursor_position();
    place_overlay(&overlay, x, y);
    let _ = overlay.set_always_on_top(true);
    let _ = overlay.show();

    let cancel = Arc::new(AtomicBool::new(false));
    let handle = app.clone();
    let thread_cancel = cancel.clone();
    let thread = std::thread::Builder::new()
        .name("autoclicker-cursor".into())
        .spawn(move || run(handle, overlay, thread_cancel))
        .map_err(|error| AppError::Internal(error.to_string()))?;

    *guard = Some(CursorWatch { cancel, thread });
    Ok(())
}

pub fn stop(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let handle = {
        let mut guard = state
            .cursor_watch
            .lock()
            .map_err(|_| AppError::Internal("cursor watch lock poisoned".into()))?;
        guard.take()
    };

    if let Some(handle) = handle {
        handle.cancel.store(true, Ordering::SeqCst);
        let _ = handle.thread.join();
    }

    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = overlay.hide();
    }
    Ok(())
}

fn run(app: AppHandle, overlay: WebviewWindow, cancel: Arc<AtomicBool>) {
    let mut last = (i32::MIN, i32::MIN);

    while !cancel.load(Ordering::Relaxed) {
        let (x, y) = inject::cursor_position();
        if (x, y) != last {
            last = (x, y);

            let scale = inject::scale_at(x, y).max(0.01);
            let payload = CursorPayload {
                physical_x: x,
                physical_y: y,
                logical_x: (x as f64 / scale).round() as i32,
                logical_y: (y as f64 / scale).round() as i32,
                scale,
            };
            let _ = app.emit(EVENT_NAME, payload);
            place_overlay(&overlay, x, y);
        }
        std::thread::sleep(TICK);
    }

    let _ = overlay.hide();
}

/// Park the overlay next to the pointer, flipping sides near screen edges.
fn place_overlay(window: &WebviewWindow, x: i32, y: i32) {
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };

    let mut left = x + OVERLAY_GAP;
    if left + OVERLAY_W > vx + vw {
        left = x - OVERLAY_GAP - OVERLAY_W;
    }

    let mut top = y + OVERLAY_GAP;
    if top + OVERLAY_H > vy + vh {
        top = y - OVERLAY_GAP - OVERLAY_H;
    }

    let _ = window.set_position(PhysicalPosition::new(left, top));
}
