use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const LOG_CAPACITY: usize = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    #[default]
    Left,
    Middle,
    Right,
    X1,
    X2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClickType {
    #[default]
    Single,
    Double,
    Hold,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PositionMode {
    #[default]
    Cursor,
    Fixed,
}

/// When the on-screen cursor readout follows the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CursorTrackMode {
    /// Always visible while the app runs.
    Always,
    /// Only while the click engine is running.
    #[default]
    Running,
    /// Never visible.
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntervalCurve {
    #[default]
    Constant,
    Linear,
    Random,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SequenceStep {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub click_type: ClickType,
    pub hold_ms: u64,
    pub delay_ms: u64,
    pub enabled: bool,
}

impl Default for SequenceStep {
    fn default() -> Self {
        Self {
            id: String::from("step"),
            x: 0,
            y: 0,
            button: MouseButton::Left,
            click_type: ClickType::Single,
            hold_ms: 40,
            delay_ms: 200,
            enabled: true,
        }
    }
}

/// Authoritative settings shared by the UI and the click engine.
/// Coordinates are physical pixels; the UI converts for display.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub interval_ms: u64,
    pub limit_enabled: bool,
    pub click_limit: u32,
    pub button: MouseButton,
    pub click_type: ClickType,
    pub hold_ms: u64,
    pub position_mode: PositionMode,
    pub pos_x: i32,
    pub pos_y: i32,
    pub cursor_track_mode: CursorTrackMode,
    pub countdown_sec: u32,
    pub jitter_interval_ms: u32,
    pub jitter_position_px: u32,
    pub curve: IntervalCurve,
    pub curve_end_ms: u64,
    pub curve_min_ms: u64,
    pub curve_max_ms: u64,
    pub sequence_enabled: bool,
    pub sequence_loop: bool,
    pub sequence: Vec<SequenceStep>,
    pub schedule_delay_sec: u32,
    pub schedule_stop_sec: u32,
    pub keyboard_enabled: bool,
    pub keyboard_key: String,
    pub window_bind_enabled: bool,
    pub window_title: String,
    pub window_process: String,
    pub toggle_hotkey: String,
    pub abort_hotkey: String,
    pub theme: String,
    pub language: String,
    pub autostart: bool,
    pub close_to_tray: bool,
    pub notify_finish: bool,
    pub update_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_ms: 100,
            limit_enabled: false,
            click_limit: 100,
            button: MouseButton::Left,
            click_type: ClickType::Single,
            hold_ms: 40,
            position_mode: PositionMode::Cursor,
            pos_x: 0,
            pos_y: 0,
            cursor_track_mode: CursorTrackMode::Running,
            countdown_sec: 0,
            jitter_interval_ms: 0,
            jitter_position_px: 0,
            curve: IntervalCurve::Constant,
            curve_end_ms: 10,
            curve_min_ms: 50,
            curve_max_ms: 150,
            sequence_enabled: false,
            sequence_loop: true,
            sequence: Vec::new(),
            schedule_delay_sec: 0,
            schedule_stop_sec: 0,
            keyboard_enabled: false,
            keyboard_key: "Space".into(),
            window_bind_enabled: false,
            window_title: String::new(),
            window_process: String::new(),
            toggle_hotkey: "F6".into(),
            abort_hotkey: "F7".into(),
            theme: "system".into(),
            language: "system".into(),
            autostart: false,
            close_to_tray: true,
            notify_finish: false,
            update_url: String::new(),
        }
    }
}

impl Settings {
    /// Clamp everything into the documented ranges. Called on every write so
    /// the engine can trust its input.
    pub fn normalize(&mut self) {
        self.interval_ms = self.interval_ms.clamp(1, 86_400_000);
        self.click_limit = self.click_limit.clamp(1, 100_000_000);
        self.hold_ms = self.hold_ms.clamp(1, 10_000);
        self.countdown_sec = self.countdown_sec.min(10);
        self.jitter_interval_ms = self.jitter_interval_ms.min(10_000);
        self.jitter_position_px = self.jitter_position_px.min(1_000);
        self.curve_end_ms = self.curve_end_ms.clamp(1, 86_400_000);
        self.curve_min_ms = self.curve_min_ms.clamp(1, 86_400_000);
        self.curve_max_ms = self.curve_max_ms.clamp(self.curve_min_ms, 86_400_000);
        self.schedule_delay_sec = self.schedule_delay_sec.min(86_400);
        self.schedule_stop_sec = self.schedule_stop_sec.min(86_400);
        self.toggle_hotkey = self.toggle_hotkey.trim().to_string();
        self.abort_hotkey = self.abort_hotkey.trim().to_string();
        self.update_url = self.update_url.trim().to_string();
        if self.sequence.len() > 100 {
            self.sequence.truncate(100);
        }
        for step in &mut self.sequence {
            step.hold_ms = step.hold_ms.clamp(1, 10_000);
            step.delay_ms = step.delay_ms.min(600_000);
        }
    }
}

#[derive(Default)]
pub struct RuntimeStats {
    pub clicks: AtomicU64,
    pub running: AtomicBool,
    pub started_at_ms: AtomicU64,
}

impl RuntimeStats {
    pub fn reset(&self) {
        self.clicks.store(0, Ordering::SeqCst);
        self.started_at_ms.store(0, Ordering::SeqCst);
    }
}

pub struct EngineHandle {
    pub cancel: Arc<AtomicBool>,
    pub thread: JoinHandle<()>,
}

/// Background cursor sampler feeding the on-screen readout overlay.
pub struct CursorWatch {
    pub cancel: Arc<AtomicBool>,
    pub thread: JoinHandle<()>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: u64,
    pub event: String,
    pub params: Value,
    pub ts: u64,
}

#[derive(Default)]
pub struct Logs {
    seq: u64,
    entries: VecDeque<LogEntry>,
}

impl Logs {
    pub fn push(&mut self, event: &str, params: Value) -> LogEntry {
        self.seq += 1;
        let entry = LogEntry {
            id: self.seq,
            event: event.to_string(),
            params,
            ts: unix_ms(),
        };
        self.entries.push_back(entry.clone());
        while self.entries.len() > LOG_CAPACITY {
            self.entries.pop_front();
        }
        entry
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    pub toggle: String,
    pub abort: String,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            toggle: "F6".into(),
            abort: "F7".into(),
        }
    }
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub stats: Arc<RuntimeStats>,
    pub engine: Mutex<Option<EngineHandle>>,
    pub cursor_watch: Mutex<Option<CursorWatch>>,
    pub logs: Mutex<Logs>,
    pub hotkeys: Mutex<HotkeyBindings>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let hotkeys = HotkeyBindings {
            toggle: settings.toggle_hotkey.clone(),
            abort: settings.abort_hotkey.clone(),
        };
        Self {
            settings: RwLock::new(settings),
            stats: Arc::new(RuntimeStats::default()),
            engine: Mutex::new(None),
            cursor_watch: Mutex::new(None),
            logs: Mutex::new(Logs::default()),
            hotkeys: Mutex::new(hotkeys),
        }
    }

    pub fn log(&self, event: &str, params: Value) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.push(event, params);
        }
    }
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

