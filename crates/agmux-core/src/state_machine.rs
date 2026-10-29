//! Lightweight classification of process lifecycle and terminal activity.
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Running,
    WaitingInput,
    Completed,
    Failed,
    Stuck,
    Frozen,
}
impl SessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Running => "RUNNING",
            Self::WaitingInput => "WAITING_INPUT",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Stuck => "STUCK",
            Self::Frozen => "FROZEN",
        }
    }
}

/// Maintains a bounded terminal tail so prompts split across PTY reads are recognized.
pub struct Classifier {
    state: SessionState,
    last_activity: Instant,
    threshold: Duration,
    tail: Vec<u8>,
}
impl Classifier {
    pub fn new(threshold: Duration) -> Self {
        Self {
            state: SessionState::Running,
            last_activity: Instant::now(),
            threshold,
            tail: Vec::new(),
        }
    }
    pub fn state(&self) -> SessionState {
        self.state
    }
    pub fn output(&mut self, bytes: &[u8]) -> Option<(SessionState, SessionState, &'static str)> {
        if bytes.is_empty() {
            return None;
        }
        self.last_activity = Instant::now();
        self.tail.extend_from_slice(bytes);
        if self.tail.len() > 512 {
            self.tail.drain(..self.tail.len() - 512);
        }
        let text = String::from_utf8_lossy(&self.tail).to_lowercase();
        let line = text.rsplit(['\n', '\r']).next().unwrap_or("").trim();
        let prompt = line.ends_with('?')
            || line.ends_with("[y/n]")
            || line.ends_with("(y/n)")
            || (line.contains("password") && line.ends_with(':'));
        self.transition(
            if prompt {
                SessionState::WaitingInput
            } else {
                SessionState::Running
            },
            if prompt {
                "input prompt detected"
            } else {
                "terminal activity"
            },
        )
    }
    pub fn input(&mut self) -> Option<(SessionState, SessionState, &'static str)> {
        self.tail.clear();
        self.last_activity = Instant::now();
        self.transition(SessionState::Running, "input received")
    }
    pub fn deadline(&self) -> Instant {
        self.last_activity + self.threshold
    }
    pub fn timeout(&mut self) -> Option<(SessionState, SessionState, &'static str)> {
        if self.state == SessionState::Running && Instant::now() >= self.deadline() {
            self.transition(SessionState::Stuck, "inactivity threshold exceeded")
        } else {
            None
        }
    }
    pub fn exit(&mut self, code: i32) -> Option<(SessionState, SessionState, &'static str)> {
        self.transition(
            if code == 0 {
                SessionState::Completed
            } else {
                SessionState::Failed
            },
            "process exited",
        )
    }
    fn transition(
        &mut self,
        next: SessionState,
        reason: &'static str,
    ) -> Option<(SessionState, SessionState, &'static str)> {
        if next == self.state {
            return None;
        }
        let old = self.state;
        self.state = next;
        Some((old, next, reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompts_input_activity_and_exit() {
        let mut c = Classifier::new(Duration::ZERO);
        assert_eq!(c.output(b"Continue [y/"), None);
        assert_eq!(c.output(b"N] ").unwrap().1, SessionState::WaitingInput);
        assert!(c.timeout().is_none());
        assert_eq!(c.input().unwrap().1, SessionState::Running);
        assert_eq!(c.timeout().unwrap().1, SessionState::Stuck);
        assert_eq!(c.output(b"progress\n").unwrap().1, SessionState::Running);
        assert_eq!(c.exit(1).unwrap().1, SessionState::Failed);
    }
}
