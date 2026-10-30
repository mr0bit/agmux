//! Configurable debounce and settle timers (`idle_after_ms` and `blocked_settle_ms`).

use std::time::{Duration, Instant};

use crate::state_machine::SessionState;

/// Evaluates `idle_after_ms` and `blocked_settle_ms` settle windows before committing heuristic transitions.
#[derive(Debug, Clone)]
pub struct DebounceTimer {
    idle_after: Duration,
    blocked_settle: Duration,
    last_output_at: Instant,
    prompt_detected_at: Option<Instant>,
}

impl DebounceTimer {
    /// Create a new `DebounceTimer` with `idle_after_ms` and `blocked_settle_ms`.
    pub fn new(idle_after_ms: u64, blocked_settle_ms: u64) -> Self {
        Self {
            idle_after: Duration::from_millis(idle_after_ms),
            blocked_settle: Duration::from_millis(blocked_settle_ms),
            last_output_at: Instant::now(),
            prompt_detected_at: None,
        }
    }

    /// Record fresh terminal output and optional prompt detection at `now`.
    pub fn record_output(&mut self, now: Instant, is_prompt: bool) {
        self.last_output_at = now;
        if is_prompt {
            if self.prompt_detected_at.is_none() {
                self.prompt_detected_at = Some(now);
            }
        } else {
            self.prompt_detected_at = None;
        }
    }

    /// Update debounce durations dynamically (e.g. after config hot-reload).
    pub fn update_durations(&mut self, idle_after_ms: u64, blocked_settle_ms: u64) {
        self.idle_after = Duration::from_millis(idle_after_ms);
        self.blocked_settle = Duration::from_millis(blocked_settle_ms);
    }

    /// Evaluate whether the settle window has elapsed at `now`.
    /// - If a blocked prompt has settled for `>= blocked_settle`, returns `Some(SessionState::WaitingInput)`.
    /// - If output has been silent for `>= idle_after` and `has_active_children` is `false`, returns `Some(SessionState::Idle)`.
    /// - If `has_active_children` is `true`, locks state to `Some(SessionState::Running)` preventing false completion flips.
    pub fn evaluate(&self, now: Instant, has_active_children: bool) -> Option<SessionState> {
        if let Some(prompt_at) = self.prompt_detected_at {
            if now.duration_since(prompt_at) >= self.blocked_settle {
                return Some(SessionState::WaitingInput);
            }
            return None;
        }

        if has_active_children {
            return Some(SessionState::Running);
        }

        if now.duration_since(self.last_output_at) >= self.idle_after {
            return Some(SessionState::Idle);
        }

        None
    }
}
