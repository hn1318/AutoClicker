//! The click loop. Runs on its own thread; started and stopped only through
//! `Arc<AtomicBool>`, never through JavaScript timers.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use super::{SessionConfig, inject, wait_until};
use crate::error::AppError;
use crate::state::{AppState, RuntimeStats};

pub const TICK_INTERVAL_MS: u64 = 100;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TickEvent {
    pub clicks: u64,
    pub elapsed_ms: u64,
    pub cps: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountdownEvent {
    pub remaining_ms: u64,
    pub total_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    pub event: String,
    pub params: serde_json::Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopReason {
    Completed,
    Cancelled,
}

impl StopReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

struct LoopOutcome {
    reason: StopReason,
    clicks: u64,
}

/// Rolling one-second click-rate meter driven by the engine itself, so the UI
/// never accumulates timing error in JavaScript.
struct CpsMeter {
    started: Instant,
    samples: VecDeque<(Instant, u64)>,
}

impl CpsMeter {
    fn new(started: Instant) -> Self {
        Self {
            started,
            samples: VecDeque::with_capacity(16),
        }
    }

    fn sample(&mut self, now: Instant, clicks: u64) -> f64 {
        self.samples.push_back((now, clicks));
        while self.samples.len() > 2 {
            let Some(second) = self.samples.get(1) else { break };
            if now.duration_since(second.0) > Duration::from_millis(1_000) {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        let Some((first_time, first_clicks)) = self.samples.front().copied() else {
            return 0.0;
        };
        let window = now.duration_since(first_time).as_secs_f64();
        if window < 0.2 {
            let total = now.duration_since(self.started).as_secs_f64();
            return if total > 0.0 {
                clicks as f64 / total
            } else {
                0.0
            };
        }
        (clicks - first_clicks) as f64 / window
    }
}

pub fn spawn(
    app: AppHandle,
    config: SessionConfig,
    stats: Arc<RuntimeStats>,
    cancel: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, AppError> {
    std::thread::Builder::new()
        .name("autoclicker-engine".into())
        .spawn(move || run(app, config, stats, cancel))
        .map_err(|error| AppError::Internal(error.to_string()))
}

fn run(app: AppHandle, config: SessionConfig, stats: Arc<RuntimeStats>, cancel: Arc<AtomicBool>) {
    // 1 ms system timer resolution: without this Sleep(1) can take 15 ms.
    unsafe {
        windows::Win32::Media::timeBeginPeriod(1);
    }

    stats.reset();
    stats.running.store(true, Ordering::SeqCst);
    stats
        .started_at_ms
        .store(crate::state::unix_ms(), Ordering::SeqCst);

    let result = run_inner(&app, &config, &stats, &cancel);

    stats.running.store(false, Ordering::SeqCst);
    unsafe {
        windows::Win32::Media::timeEndPeriod(1);
    }

    match result {
        Ok(outcome) => {
            let payload = json!({
                "reason": outcome.reason.as_str(),
                "clicks": outcome.clicks,
            });
            log(&app, "stopped", payload.clone());
            emit_event(&app, "stopped", payload);
        }
        Err(error) => {
            let payload = json!({ "code": error.code(), "params": error.params() });
            log(&app, "error", payload.clone());
            emit_event(&app, "error", payload);
        }
    }
}

fn run_inner(
    app: &AppHandle,
    config: &SessionConfig,
    stats: &Arc<RuntimeStats>,
    cancel: &Arc<AtomicBool>,
) -> Result<LoopOutcome, AppError> {
    let total_delay = config.countdown_ms + config.schedule_delay_ms;
    if total_delay > 0 {
        if config.countdown_ms > 0 {
            emit_event(app, "countdownStarted", json!({ "ms": config.countdown_ms }));
        }
        if config.schedule_delay_ms > 0 {
            emit_event(app, "scheduled", json!({ "ms": config.schedule_delay_ms }));
        }

        let mut remaining = total_delay;
        while remaining > 0 {
            if cancel.load(Ordering::Relaxed) {
                return Ok(LoopOutcome {
                    reason: StopReason::Cancelled,
                    clicks: 0,
                });
            }
            let _ = app.emit(
                "runtime:countdown",
                CountdownEvent {
                    remaining_ms: remaining,
                    total_ms: total_delay,
                },
            );
            let slice = remaining.min(100);
            std::thread::sleep(Duration::from_millis(slice));
            remaining -= slice;
        }
        let _ = app.emit(
            "runtime:countdown",
            CountdownEvent {
                remaining_ms: 0,
                total_ms: total_delay,
            },
        );
    }

    emit_event(
        app,
        "started",
        json!({
            "intervalMs": config.interval_ms,
            "limit": config.limit,
            "mode": if config.sequence.is_some() { "sequence" } else if config.keyboard_vk.is_some() { "keyboard" } else { "point" },
        }),
    );

    let started = Instant::now();
    if let Some(plan) = &config.sequence {
        run_sequence(app, config, plan, stats, cancel, started)
    } else {
        run_single(app, config, stats, cancel, started)
    }
}

fn run_single(
    app: &AppHandle,
    config: &SessionConfig,
    stats: &Arc<RuntimeStats>,
    cancel: &Arc<AtomicBool>,
    started: Instant,
) -> Result<LoopOutcome, AppError> {
    let mut next = started;
    let mut clicks = 0u64;
    let mut meter = CpsMeter::new(started);
    let mut last_emit = started;
    let mut filter_ok = true;
    let mut last_filter_check = started;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(LoopOutcome {
                reason: StopReason::Cancelled,
                clicks,
            });
        }

        if let Some(filter) = &config.window_filter {
            if last_filter_check.elapsed() >= Duration::from_millis(120) {
                filter_ok = filter.matches();
                last_filter_check = Instant::now();
            }
            if !filter_ok {
                emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);
                if inject::sleep_cancellable(cancel, 50) {
                    return Ok(LoopOutcome {
                        reason: StopReason::Cancelled,
                        clicks,
                    });
                }
                // Resume without a burst of catch-up clicks.
                next = Instant::now();
                continue;
            }
        }

        if !wait_until(next, cancel) {
            return Ok(LoopOutcome {
                reason: StopReason::Cancelled,
                clicks,
            });
        }

        let target = config.target_point();

        // Never inject into our own window: with "follow cursor" the loop would
        // otherwise keep clicking the UI, and every injected down/up pair resets
        // the Stop button before the user's physical click registers.
        if config.keyboard_vk.is_none() {
            let probe = target.unwrap_or_else(inject::cursor_position);
            if inject::point_in_own_window(probe.0, probe.1) {
                emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);
                if inject::sleep_cancellable(cancel, 50) {
                    return Ok(LoopOutcome {
                        reason: StopReason::Cancelled,
                        clicks,
                    });
                }
                // Resume from now so leaving the window causes no click burst.
                next = Instant::now();
                continue;
            }
        }

        let action = if let Some(vk) = config.keyboard_vk {
            inject::press_key(vk, config.hold_ms, cancel)?
        } else {
            inject::click(
                config.button,
                config.click_type,
                config.hold_ms,
                target,
                cancel,
            )?
        };
        if !action {
            return Ok(LoopOutcome {
                reason: StopReason::Cancelled,
                clicks,
            });
        }

        clicks += 1;
        stats.clicks.store(clicks, Ordering::Relaxed);
        emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);

        if let Some(limit) = config.limit {
            if clicks >= limit as u64 {
                return Ok(LoopOutcome {
                    reason: StopReason::Completed,
                    clicks,
                });
            }
        }
        if let Some(stop_ms) = config.schedule_stop_ms {
            if started.elapsed().as_millis() as u64 >= stop_ms {
                return Ok(LoopOutcome {
                    reason: StopReason::Completed,
                    clicks,
                });
            }
        }

        let interval = config.interval_at(clicks, started.elapsed().as_millis() as u64);
        next += Duration::from_millis(interval);
        let now = Instant::now();
        if next < now {
            // Behind schedule (injection slower than the interval): never burst.
            next = now;
        }
    }
}

fn run_sequence(
    app: &AppHandle,
    config: &SessionConfig,
    plan: &super::sequence::SequencePlan,
    stats: &Arc<RuntimeStats>,
    cancel: &Arc<AtomicBool>,
    started: Instant,
) -> Result<LoopOutcome, AppError> {
    let mut clicks = 0u64;
    let mut meter = CpsMeter::new(started);
    let mut last_emit = started;

    loop {
        for step in &plan.steps {
            if cancel.load(Ordering::Relaxed) {
                return Ok(LoopOutcome {
                    reason: StopReason::Cancelled,
                    clicks,
                });
            }

            let point = config.jitter_point(step.x, step.y);
            if inject::point_in_own_window(point.0, point.1) {
                emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);
                if inject::sleep_cancellable(cancel, 50) {
                    return Ok(LoopOutcome {
                        reason: StopReason::Cancelled,
                        clicks,
                    });
                }
                continue;
            }

            let target = Some(point);
            let action = inject::click(step.button, step.click_type, step.hold_ms, target, cancel)?;
            if !action {
                return Ok(LoopOutcome {
                    reason: StopReason::Cancelled,
                    clicks,
                });
            }

            clicks += 1;
            stats.clicks.store(clicks, Ordering::Relaxed);
            emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);

            if let Some(limit) = config.limit {
                if clicks >= limit as u64 {
                    return Ok(LoopOutcome {
                        reason: StopReason::Completed,
                        clicks,
                    });
                }
            }
            if let Some(stop_ms) = config.schedule_stop_ms {
                if started.elapsed().as_millis() as u64 >= stop_ms {
                    return Ok(LoopOutcome {
                        reason: StopReason::Completed,
                        clicks,
                    });
                }
            }

            let jitter = config.jitter_interval_ms as i64;
            let step_delay = if jitter > 0 {
                (step.delay_ms as i64 + fastrand::i64(-jitter..=jitter)).max(0) as u64
            } else {
                step.delay_ms
            };

            let deadline = Instant::now() + Duration::from_millis(step_delay);
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(LoopOutcome {
                        reason: StopReason::Cancelled,
                        clicks,
                    });
                }
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let slice = (deadline - now).as_millis().min(20) as u64;
                std::thread::sleep(Duration::from_millis(slice));
                emit_tick_if_due(app, &mut last_emit, &mut meter, clicks, started);
            }
        }

        if !plan.loop_enabled {
            return Ok(LoopOutcome {
                reason: StopReason::Completed,
                clicks,
            });
        }
    }
}

fn emit_tick_if_due(
    app: &AppHandle,
    last_emit: &mut Instant,
    meter: &mut CpsMeter,
    clicks: u64,
    started: Instant,
) {
    let now = Instant::now();
    if now.duration_since(*last_emit) < Duration::from_millis(TICK_INTERVAL_MS) {
        return;
    }
    *last_emit = now;
    let payload = TickEvent {
        clicks,
        elapsed_ms: now.duration_since(started).as_millis() as u64,
        cps: meter.sample(now, clicks),
    };
    let _ = app.emit("runtime:tick", payload);
}

fn emit_event(app: &AppHandle, event: &str, params: serde_json::Value) {
    let _ = app.emit(
        "runtime:event",
        RunEvent {
            event: event.to_string(),
            params,
        },
    );
}

fn log(app: &AppHandle, event: &str, params: serde_json::Value) {
    if let Some(state) = app.try_state::<AppState>() {
        state.log(event, params);
    }
}
