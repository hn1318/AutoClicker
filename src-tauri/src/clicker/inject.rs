//! Direct SendInput wrapper.
//!
//! Deliberately hand-rolled instead of using enigo: we need exact control over
//! X1/X2 `mouseData`, batched down/up pairs (so a 1 ms interval stays feasible)
//! and physical-pixel absolute moves across the whole virtual desktop.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::Foundation::{POINT, GetLastError};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetDesktopWindow, GetSystemMetrics, GetWindowThreadProcessId,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WindowFromPoint,
    XBUTTON1, XBUTTON2,
};

use crate::error::AppError;
use crate::state::{ClickType, MouseButton};

const DOUBLE_CLICK_GAP_MS: u64 = 40;
const ERROR_ACCESS_DENIED: u32 = 5;

fn mouse_input(flags: MOUSE_EVENT_FLAGS, data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_input(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), AppError> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        return Ok(());
    }
    let code = unsafe { GetLastError().0 };
    if code == ERROR_ACCESS_DENIED {
        // UIPI: the foreground window belongs to a higher integrity level.
        Err(AppError::InputBlocked)
    } else {
        Err(AppError::Platform(format!("SendInput failed (win32 {code})")))
    }
}

/// Move the hardware cursor to a physical-pixel position on the virtual desktop.
pub fn move_to(x: i32, y: i32) -> Result<(), AppError> {
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        )
    };
    let nx = (((x - vx) as f64) * 65535.0 / ((vw - 1).max(1) as f64)).round() as i32;
    let ny = (((y - vy) as f64) * 65535.0 / ((vh - 1).max(1) as f64)).round() as i32;

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: nx,
                dy: ny,
                mouseData: 0,
                dwFlags: MOUSE_EVENT_FLAGS(
                    MOUSEEVENTF_MOVE.0 | MOUSEEVENTF_ABSOLUTE.0 | MOUSEEVENTF_VIRTUALDESK.0,
                ),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    send(&[input])
}

fn button_flags(button: MouseButton) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS, u32) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        MouseButton::X1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON1 as u32),
        MouseButton::X2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON2 as u32),
    }
}

/// True when `(x, y)` (physical px) lands on a window owned by this process.
///
/// The engine must never inject into its own window: with "follow cursor"
/// enabled the click loop would otherwise keep hitting whatever sits under the
/// pointer — including the Stop button, which then becomes impossible to
/// press because every injected down/up pair resets the button state.
pub fn point_in_own_window(x: i32, y: i32) -> bool {
    unsafe {
        let hwnd = WindowFromPoint(POINT { x, y });
        if hwnd.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid != 0 && pid == GetCurrentProcessId()
    }
}

/// Sleep in small slices so a stop request is honoured within ~5 ms.
/// Returns `true` when cancelled.
pub fn sleep_cancellable(cancel: &AtomicBool, ms: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return true;
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return false;
        }
        let remaining = (deadline - now).as_millis() as u64;
        std::thread::sleep(Duration::from_millis(remaining.min(5)));
    }
}

/// Perform one click action at `target` (physical px) or at the current cursor.
/// Returns `Ok(false)` when the action was interrupted by a stop request.
pub fn click(
    button: MouseButton,
    click_type: ClickType,
    hold_ms: u64,
    target: Option<(i32, i32)>,
    cancel: &AtomicBool,
) -> Result<bool, AppError> {
    if let Some((x, y)) = target {
        if cancel.load(Ordering::Relaxed) {
            return Ok(false);
        }
        move_to(x, y)?;
    }

    let (down, up, data) = button_flags(button);

    match click_type {
        // Batched press + release: no artificial gap, keeps 1 ms intervals viable.
        ClickType::Single => {
            send(&[mouse_input(down, data), mouse_input(up, data)])?;
            Ok(true)
        }
        ClickType::Double => {
            send(&[mouse_input(down, data), mouse_input(up, data)])?;
            if sleep_cancellable(cancel, DOUBLE_CLICK_GAP_MS) {
                return Ok(false);
            }
            send(&[mouse_input(down, data), mouse_input(up, data)])?;
            Ok(true)
        }
        ClickType::Hold => {
            send(&[mouse_input(down, data)])?;
            let cancelled = sleep_cancellable(cancel, hold_ms);
            // Always release, even when cancelled, so the button never sticks.
            send(&[mouse_input(up, data)])?;
            Ok(!cancelled)
        }
    }
}

/// Press a virtual key. Returns `Ok(false)` when interrupted by a stop request.
pub fn press_key(vk: u16, hold_ms: u64, cancel: &AtomicBool) -> Result<bool, AppError> {
    send(&[
        key_input(vk, KEYBD_EVENT_FLAGS(0)),
        key_input(vk, KEYEVENTF_KEYUP),
    ])?;
    if hold_ms > 0 && cancel.load(Ordering::Relaxed) {
        return Ok(false);
    }
    Ok(true)
}

pub fn cursor_position() -> (i32, i32) {
    let mut point = POINT::default();
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok.is_err() {
        return (0, 0);
    }
    (point.x, point.y)
}

/// Scale factor of the monitor that contains the given physical point.
pub fn scale_at(x: i32, y: i32) -> f64 {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok()
            && dpi_x > 0
        {
            return dpi_x as f64 / 96.0;
        }
        let window_dpi = GetDpiForWindow(GetDesktopWindow());
        if window_dpi > 0 {
            window_dpi as f64 / 96.0
        } else {
            1.0
        }
    }
}

/// Map a human readable key name to a Windows virtual key code.
pub fn virtual_key(name: &str) -> Option<u16> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 16 {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();

    if upper.len() == 1 {
        let ch = upper.chars().next()?;
        if ch.is_ascii_alphanumeric() {
            return Some(ch as u16);
        }
    }

    if let Some(rest) = upper.strip_prefix('F') {
        if let Ok(index) = rest.parse::<u16>() {
            if (1..=24).contains(&index) {
                return Some(0x70 + index - 1);
            }
        }
    }

    let vk = match upper.as_str() {
        "SPACE" => 0x20,
        "ENTER" | "RETURN" => 0x0D,
        "TAB" => 0x09,
        "ESC" | "ESCAPE" => 0x1B,
        "BACKSPACE" => 0x08,
        "DELETE" | "DEL" => 0x2E,
        "INSERT" | "INS" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" | "PGUP" => 0x21,
        "PAGEDOWN" | "PGDN" => 0x22,
        "ARROWLEFT" | "LEFT" => 0x25,
        "ARROWUP" | "UP" => 0x26,
        "ARROWRIGHT" | "RIGHT" => 0x27,
        "ARROWDOWN" | "DOWN" => 0x28,
        "SHIFT" => 0x10,
        "CTRL" | "CONTROL" => 0x11,
        "ALT" => 0x12,
        "WIN" | "META" | "SUPER" => 0x5B,
        "CAPSLOCK" => 0x14,
        "NUMLOCK" => 0x90,
        _ => return None,
    };
    Some(vk)
}
