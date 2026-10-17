pub mod engine;
pub mod inject;
pub mod sequence;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::error::AppError;
use crate::state::{
    ClickType, IntervalCurve, MouseButton, PositionMode, SequenceStep, Settings,
};
use crate::window_bind::WindowFilter;

use sequence::SequencePlan;

/// Immutable snapshot handed to the engine thread when a run starts.
/// Settings edited mid-run therefore never race with the click loop.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub interval_ms: u64,
    pub limit: Option<u32>,
    pub button: MouseButton,
    pub click_type: ClickType,
    pub hold_ms: u64,
    pub position: PositionTarget,
    pub countdown_ms: u64,
    pub schedule_delay_ms: u64,
    pub schedule_stop_ms: Option<u64>,
    pub jitter_interval_ms: u32,
    pub jitter_position_px: u32,
    pub curve: IntervalCurve,
    pub curve_end_ms: u64,
    pub curve_min_ms: u64,
    pub curve_max_ms: u64,
    pub sequence: Option<SequencePlan>,
    pub keyboard_vk: Option<u16>,
    pub window_filter: Option<WindowFilter>,
}

#[derive(Clone, Copy, Debug)]
pub enum PositionTarget {
    Cursor,
    Fixed(i32, i32),
}

impl SessionConfig {
    pub fn from_settings(settings: &Settings) -> Result<Self, AppError> {
        if settings.interval_ms == 0 {
            return Err(AppError::InvalidInput("intervalMs"));
        }
        if settings.hold_ms == 0 {
            return Err(AppError::InvalidInput("holdMs"));
        }

        let limit = if settings.limit_enabled {
            if settings.click_limit == 0 {
                return Err(AppError::InvalidInput("clickLimit"));
            }
            Some(settings.click_limit)
        } else {
            None
        };

        let keyboard_vk = if settings.keyboard_enabled {
            Some(
                inject::virtual_key(&settings.keyboard_key)
                    .ok_or(AppError::InvalidInput("keyboardKey"))?,
            )
        } else {
            None
        };

        // Keyboard mode replaces mouse clicking entirely, so sequences are ignored.
        let sequence = if settings.sequence_enabled && keyboard_vk.is_none() {
            let steps: Vec<SequenceStep> = settings
                .sequence
                .iter()
                .filter(|step| step.enabled)
                .cloned()
                .collect();
            Some(
                SequencePlan::new(steps, settings.sequence_loop)
                    .ok_or(AppError::InvalidInput("sequence"))?,
            )
        } else {
            None
        };

        let window_filter =
            if settings.window_bind_enabled && !settings.window_title.trim().is_empty()
                || settings.window_bind_enabled && !settings.window_process.trim().is_empty()
            {
                Some(WindowFilter {
                    title: settings.window_title.trim().to_string(),
                    process: settings.window_process.trim().to_string(),
                })
            } else {
                None
            };

        let position = match settings.position_mode {
            PositionMode::Cursor => PositionTarget::Cursor,
            PositionMode::Fixed => PositionTarget::Fixed(settings.pos_x, settings.pos_y),
        };

        Ok(Self {
            interval_ms: settings.interval_ms,
            limit,
            button: settings.button,
            click_type: settings.click_type,
            hold_ms: settings.hold_ms,
            position,
            countdown_ms: settings.countdown_sec as u64 * 1000,
            schedule_delay_ms: settings.schedule_delay_sec as u64 * 1000,
            schedule_stop_ms: if settings.schedule_stop_sec > 0 {
                Some(settings.schedule_stop_sec as u64 * 1000)
            } else {
                None
            },
            jitter_interval_ms: settings.jitter_interval_ms,
            jitter_position_px: settings.jitter_position_px,
            curve: settings.curve,
            curve_end_ms: settings.curve_end_ms,
            curve_min_ms: settings.curve_min_ms,
            curve_max_ms: settings.curve_max_ms,
            sequence,
            keyboard_vk,
            window_filter,
        })
    }

    /// Interval in milliseconds for the next click, including curve + jitter.
    pub fn interval_at(&self, clicks: u64, elapsed_ms: u64) -> u64 {
        let base = match self.curve {
            IntervalCurve::Constant => self.interval_ms as f64,
            IntervalCurve::Linear => {
                let progress = if let Some(limit) = self.limit {
                    (clicks as f64 / limit.max(1) as f64).min(1.0)
                } else if let Some(stop) = self.schedule_stop_ms {
                    (elapsed_ms as f64 / stop.max(1) as f64).min(1.0)
                } else {
                    // No finite run: ramp over the first 600 clicks, then hold.
                    (clicks as f64 / 600.0).min(1.0)
                };
                let start = self.interval_ms as f64;
                let end = self.curve_end_ms as f64;
                start + (end - start) * progress
            }
            IntervalCurve::Random => {
                let min = self.curve_min_ms.min(self.curve_max_ms);
                let max = self.curve_max_ms.max(self.curve_min_ms);
                fastrand::u64(min..=max) as f64
            }
        };

        let jitter = self.jitter_interval_ms as i64;
        let value = if jitter > 0 {
            base + fastrand::i64(-jitter..=jitter) as f64
        } else {
            base
        };

        value.round().max(1.0) as u64
    }

    /// Target point in physical pixels, or `None` to click wherever the cursor is.
    pub fn target_point(&self) -> Option<(i32, i32)> {
        match self.position {
            PositionTarget::Cursor => None,
            PositionTarget::Fixed(x, y) => Some(self.jitter_point(x, y)),
        }
    }

    pub fn jitter_point(&self, x: i32, y: i32) -> (i32, i32) {
        let jitter = self.jitter_position_px as i32;
        if jitter <= 0 {
            return (x, y);
        }
        (
            x + fastrand::i32(-jitter..=jitter),
            y + fastrand::i32(-jitter..=jitter),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_keys_parse() {
        assert_eq!(inject::virtual_key("F6"), Some(0x75));
        assert_eq!(inject::virtual_key("f6"), Some(0x75));
        assert_eq!(inject::virtual_key("A"), Some(0x41));
        assert_eq!(inject::virtual_key("9"), Some(0x39));
        assert_eq!(inject::virtual_key("Space"), Some(0x20));
        assert_eq!(inject::virtual_key("not-a-key"), None);
        assert_eq!(inject::virtual_key(""), None);
    }

    #[test]
    fn jittered_interval_never_drops_below_one_ms() {
        let mut settings = Settings::default();
        settings.interval_ms = 1;
        settings.jitter_interval_ms = 50;
        let config = SessionConfig::from_settings(&settings).expect("valid config");
        for _ in 0..1_000 {
            let value = config.interval_at(1, 10);
            assert!((1..=51).contains(&value), "unexpected interval {value}");
        }
    }

    #[test]
    fn linear_curve_interpolates_between_start_and_end() {
        let mut settings = Settings::default();
        settings.interval_ms = 100;
        settings.curve = IntervalCurve::Linear;
        settings.curve_end_ms = 10;
        settings.limit_enabled = true;
        settings.click_limit = 10;
        let config = SessionConfig::from_settings(&settings).expect("valid config");
        assert_eq!(config.interval_at(0, 0), 100);
        assert_eq!(config.interval_at(10, 0), 10);
    }

    #[test]
    fn sequence_mode_requires_an_enabled_step() {
        let mut settings = Settings::default();
        settings.sequence_enabled = true;
        assert!(SessionConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn keyboard_mode_ignores_sequences_and_reports_invalid_key() {
        let mut settings = Settings::default();
        settings.keyboard_enabled = true;
        settings.keyboard_key = "NotARealKey".into();
        assert!(SessionConfig::from_settings(&settings).is_err());

        settings.keyboard_key = "Space".into();
        settings.sequence_enabled = true;
        let config = SessionConfig::from_settings(&settings).expect("valid config");
        assert!(config.sequence.is_none());
        assert_eq!(config.keyboard_vk, Some(0x20));
    }
}

/// Absolute-deadline wait. Returns `false` when a stop request arrived.
///
/// Strategy: sleep in ≤5 ms slices while far away, then spin for the last
/// ~1.2 ms. Combined with `timeBeginPeriod(1)` this holds a 1 ms interval
/// without overshooting (Sleep(1) can otherwise cost 2 ms).
pub fn wait_until(deadline: std::time::Instant, cancel: &AtomicBool) -> bool {
    loop {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return true;
        }
        let remaining = deadline - now;
        let micros = remaining.as_micros();
        if micros > 2_000 {
            let sleep_ms = ((micros - 1_000) / 1_000).min(5).max(1) as u64;
            std::thread::sleep(Duration::from_millis(sleep_ms));
        } else {
            std::hint::spin_loop();
        }
    }
}
