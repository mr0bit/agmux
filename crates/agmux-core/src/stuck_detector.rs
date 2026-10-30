//! Tool-specific stuck prompt heuristic (`REQ-PLUGIN-STUCK-HEURISTIC-002`).

/// Default timeout (`10,000` ms = 10s) before unsubmitted text at a prompt transitions to `STUCK`.
pub const DEFAULT_STUCK_TIMEOUT_MS: u64 = 10_000;

/// Evaluates whether unsubmitted text sits after a prompt marker without pressing Enter.
#[derive(Debug, Clone, Copy)]
pub struct StuckDetector {
    pub stuck_timeout_ms: u64,
}

impl Default for StuckDetector {
    fn default() -> Self {
        Self {
            stuck_timeout_ms: DEFAULT_STUCK_TIMEOUT_MS,
        }
    }
}

impl StuckDetector {
    /// Create a new `StuckDetector` with a custom timeout in milliseconds.
    pub const fn new(stuck_timeout_ms: u64) -> Self {
        Self { stuck_timeout_ms }
    }

    /// Check whether any of the bottom visible screen lines contains `prompt_prefix` followed by
    /// non-whitespace unsubmitted text, while both `last_input_elapsed_ms` and `pty_silence_ms`
    /// have reached `stuck_timeout_ms` and `has_active_children` is `false`.
    pub fn is_stuck(
        &self,
        prompt_prefix: &str,
        screen_lines: &[&str],
        last_input_elapsed_ms: u64,
        pty_silence_ms: u64,
        has_active_children: bool,
    ) -> bool {
        if has_active_children {
            return false;
        }
        if last_input_elapsed_ms < self.stuck_timeout_ms || pty_silence_ms < self.stuck_timeout_ms {
            return false;
        }

        let start = screen_lines.len().saturating_sub(5);
        for line in screen_lines[start..].iter().rev() {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = strip_prompt_marker(trimmed, prompt_prefix) {
                if !rest.trim().is_empty() {
                    return true;
                }
            }
        }
        false
    }
}

fn strip_prompt_marker<'a>(line: &'a str, prompt_prefix: &str) -> Option<&'a str> {
    let prefix_trimmed = prompt_prefix.trim();
    if !prefix_trimmed.is_empty()
        && let Some(rest) = line.strip_prefix(prefix_trimmed)
    {
        return Some(rest);
    }
    for fallback in ["claude>", "aider>", "codex>", "agy>", "gemini>", "❯", ">"] {
        if let Some(rest) = line.strip_prefix(fallback) {
            return Some(rest);
        }
    }
    None
}
