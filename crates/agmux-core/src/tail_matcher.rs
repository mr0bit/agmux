//! Tier 3 VT100 bottom-5 row pattern matcher for spinners, rate-limit banners, and interactive prompts.

use crate::state_machine::SessionState;

/// Number of bottom visible rows inspected by `TailMatcher`.
pub const TAIL_ROWS: usize = 5;

/// Unicode Braille spinner characters (`U+2800..=U+28FF` common agent spinner frames).
pub const BRAILLE_SPINNERS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Classification result produced by inspecting the bottom visible rows of the VT100 screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailMatchResult {
    /// Active Braille spinner detected -> `SessionState::Running` (`WORKING`).
    SpinnerWorking,
    /// Rate-limit or quota exhaustion banner detected -> `SessionState::Failed` (`ERRORED`).
    RateLimitOrQuotaError,
    /// Permission or confirmation prompt detected -> `SessionState::WaitingInput`.
    PromptWaitingInput,
    /// No special pattern matched in bottom 5 rows.
    None,
}

impl TailMatchResult {
    /// Map `TailMatchResult` to `Option<SessionState>`.
    pub fn to_session_state(self) -> Option<SessionState> {
        match self {
            Self::SpinnerWorking => Some(SessionState::Running),
            Self::RateLimitOrQuotaError => Some(SessionState::Failed),
            Self::PromptWaitingInput => Some(SessionState::WaitingInput),
            Self::None => None,
        }
    }
}

/// Inspects the bottom 5 visible rows of a VT100 screen buffer.
#[derive(Debug, Clone, Default)]
pub struct TailMatcher;

impl TailMatcher {
    /// Classify a slice of visible screen lines by inspecting at most the bottom `TAIL_ROWS` (5) non-empty lines.
    pub fn classify_lines(lines: &[String]) -> TailMatchResult {
        let start = lines.len().saturating_sub(TAIL_ROWS);
        let tail_slice = &lines[start..];

        for line in tail_slice.iter().rev() {
            let lower = line.to_lowercase();
            if lower.contains("rate limit exceeded")
                || lower.contains("429 too many requests")
                || lower.contains("quota exceeded")
                || lower.contains("insufficient_quota")
                || lower.contains("usage limit reached")
            {
                return TailMatchResult::RateLimitOrQuotaError;
            }
        }

        for line in tail_slice.iter().rev() {
            let trimmed = line.trim();
            let lower = trimmed.to_lowercase();
            if lower.contains("(y/n)")
                || lower.contains("[y/n]")
                || lower.contains("allow?")
                || lower.contains("approve?")
                || lower.contains("do you want to proceed")
                || (lower.contains("password") && lower.ends_with(':'))
            {
                return TailMatchResult::PromptWaitingInput;
            }
        }

        for line in tail_slice.iter().rev() {
            if line.chars().any(|c| BRAILLE_SPINNERS.contains(&c)) {
                return TailMatchResult::SpinnerWorking;
            }
        }

        TailMatchResult::None
    }
}
