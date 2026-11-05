//! Records real mouse clicks into sequence steps.
//!
//! A `WH_MOUSE_LL` hook needs a message pump, so it lives on a dedicated thread
//! that is torn down with `PostThreadMessageW(WM_QUIT)`. Injected events are
//! filtered out (`LLMHF_INJECTED`) and so are clicks landing on our own window,
//! which keeps replay from recording itself and keeps the stop button out of the
//! capture.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN, WM_XBUTTONDOWN, XBUTTON1, XBUTTON2,
};

use super::inject;
use crate::error::AppError;
use crate::state::{AppState, MouseButton, RecordedClick, RecorderHandle};

struct HookState {
    buffer: Arc<Mutex<Vec<RecordedClick>>>,
    started: Instant,
}

/// Shared with the hook callback, which cannot capture environment.
static HOOK_STATE: Mutex<Option<HookState>> = Mutex::new(None);

pub fn is_recording(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .recorder
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

pub fn start(app: &AppHandle) -> Result<(), AppError> {
    // Restarting while already recording simply drops the previous capture.
    let _ = stop(app)?;

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let thread_id = Arc::new(AtomicU32::new(0));

    {
        let mut guard = HOOK_STATE
            .lock()
            .map_err(|_| AppError::Internal("recorder state poisoned".into()))?;
        *guard = Some(HookState {
            buffer: buffer.clone(),
            started: Instant::now(),
        });
    }

    let tid = thread_id.clone();
    let thread = std::thread::Builder::new()
        .name("autoclicker-recorder".into())
        .spawn(move || pump(tid))
        .map_err(|error| AppError::Internal(error.to_string()))?;

    // The pump publishes its thread id once the hook is installed; a zero after
    // this window means `SetWindowsHookEx` failed.
    for _ in 0..30 {
        if thread_id.load(Ordering::SeqCst) != 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    if thread_id.load(Ordering::SeqCst) == 0 {
        let _ = thread.join();
        if let Ok(mut hook) = HOOK_STATE.lock() {
            *hook = None;
        }
        return Err(AppError::Internal("recorder hook failed to install".into()));
    }

    let state = app.state::<AppState>();
    let mut guard = state
        .recorder
        .lock()
        .map_err(|_| AppError::Internal("recorder state poisoned".into()))?;
    *guard = Some(RecorderHandle {
        thread,
        thread_id,
        buffer,
    });

    Ok(())
}

pub fn stop(app: &AppHandle) -> Result<Vec<RecordedClick>, AppError> {
    let handle = {
        let state = app.state::<AppState>();
        let mut guard = state
            .recorder
            .lock()
            .map_err(|_| AppError::Internal("recorder state poisoned".into()))?;
        guard.take()
    };

    let Some(handle) = handle else {
        return Ok(Vec::new());
    };

    // The hook thread publishes its id once the hook is installed.
    for _ in 0..50 {
        if handle.thread_id.load(Ordering::SeqCst) != 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let id = handle.thread_id.load(Ordering::SeqCst);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
    let _ = handle.thread.join();

    let clicks = handle
        .buffer
        .lock()
        .map(|buffer| buffer.clone())
        .unwrap_or_default();
    Ok(clicks)
}

/// Message pump required by low-level hooks.
fn pump(thread_id: Arc<AtomicU32>) {
    unsafe {
        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), None, 0) {
            Ok(hook) => hook,
            Err(_) => {
                if let Ok(mut state) = HOOK_STATE.lock() {
                    *state = None;
                }
                return;
            }
        };

        thread_id.store(GetCurrentThreadId(), Ordering::SeqCst);

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = UnhookWindowsHookEx(hook);
    }

    if let Ok(mut state) = HOOK_STATE.lock() {
        *state = None;
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        if info.flags & LLMHF_INJECTED == 0 {
            if let Some(button) = button_of(wparam, info) {
                record(info.pt.x, info.pt.y, button);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn button_of(wparam: WPARAM, info: &MSLLHOOKSTRUCT) -> Option<MouseButton> {
    match wparam.0 as u32 {
        WM_LBUTTONDOWN => Some(MouseButton::Left),
        WM_RBUTTONDOWN => Some(MouseButton::Right),
        WM_MBUTTONDOWN => Some(MouseButton::Middle),
        WM_XBUTTONDOWN => match ((info.mouseData >> 16) & 0xFFFF) as u16 {
            XBUTTON1 => Some(MouseButton::X1),
            XBUTTON2 => Some(MouseButton::X2),
            _ => None,
        },
        _ => None,
    }
}

fn record(x: i32, y: i32, button: MouseButton) {
    // Clicks on our own UI (record / stop buttons) are never worth replaying.
    if inject::point_in_own_window(x, y) {
        return;
    }

    let Ok(mut guard) = HOOK_STATE.lock() else { return };
    let Some(state) = guard.as_mut() else { return };
    let elapsed = state.started.elapsed().as_millis() as u64;
    if let Ok(mut buffer) = state.buffer.lock() {
        buffer.push(RecordedClick {
            x,
            y,
            button,
            ts_ms: elapsed,
        });
    }
}
