//! Foreground-window matching used by the optional "only click while X is in
//! front" guard.

use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::PWSTR;

#[derive(Clone, Debug, Default)]
pub struct WindowFilter {
    pub title: String,
    pub process: String,
}

impl WindowFilter {
    pub fn is_disabled(&self) -> bool {
        self.title.trim().is_empty() && self.process.trim().is_empty()
    }

    pub fn matches(&self) -> bool {
        if self.is_disabled() {
            return true;
        }
        let Some(foreground) = foreground_info() else {
            return false;
        };
        let title_ok = self.title.trim().is_empty()
            || foreground
                .title
                .to_lowercase()
                .contains(&self.title.trim().to_lowercase());
        let process_ok = self.process.trim().is_empty()
            || foreground
                .process
                .to_lowercase()
                .contains(&self.process.trim().to_lowercase());
        title_ok && process_ok
    }
}

#[derive(Clone, Debug, Default)]
pub struct ForegroundInfo {
    pub title: String,
    pub process: String,
}

pub fn foreground_info() -> Option<ForegroundInfo> {
    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }

    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    let title = String::from_utf16_lossy(&buffer[..len.max(0) as usize]);

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    Some(ForegroundInfo {
        title,
        process: process_name(pid).unwrap_or_default(),
    })
}

fn process_name(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let result =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        if result.is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        path.rsplit('\\').next().map(|name| name.to_string())
    }
}
