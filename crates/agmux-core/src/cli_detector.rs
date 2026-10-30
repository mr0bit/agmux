//! Missing CLI binary detector and interactive installation guidance (`REQ-PLUGIN-MISSING-DETECTOR-006`).

use std::path::{Path, PathBuf};

use crate::builtin_plugins::is_windows_system_cmd_collision;

/// Structured installation guidance returned when a requested agent binary is missing from `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCliGuidance {
    pub binary_name: String,
    pub display_name: String,
    pub install_command: String,
    pub docs_url: String,
}

impl MissingCliGuidance {
    /// Format a user-facing banner for TUI modals or CLI error output.
    pub fn format_banner(&self) -> String {
        format!(
            "{} (`{}`) is not installed or not found on PATH.\nInstall with: {}\nDocumentation: {}",
            self.display_name, self.binary_name, self.install_command, self.docs_url
        )
    }
}

/// Checks whether an agent CLI binary is available in `PATH` (or custom search directories) before spawning.
#[derive(Debug, Clone, Default)]
pub struct CliDetector;

impl CliDetector {
    /// Verify whether `binary_name` exists in the current `PATH`.
    /// Returns `Ok(resolved_path)` if found, or `Err(MissingCliGuidance)` if missing.
    pub fn check_in_system_path(binary_name: &str) -> Result<PathBuf, MissingCliGuidance> {
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        let search_dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
        Self::check_in_dirs(binary_name, &search_dirs)
    }

    /// Verify whether `binary_name` exists in `search_dirs`.
    pub fn check_in_dirs(
        binary_name: &str,
        search_dirs: &[PathBuf],
    ) -> Result<PathBuf, MissingCliGuidance> {
        for dir in search_dirs {
            if let Some(found) = find_executable_in_dir(dir, binary_name)
                && !is_windows_system_cmd_collision(binary_name, &found)
            {
                return Ok(found);
            }
        }
        Err(Self::guidance_for(binary_name))
    }

    /// Return official installation guidance for any of the 14 built-in agents or a custom binary.
    pub fn guidance_for(binary_name: &str) -> MissingCliGuidance {
        let (display_name, install_command, docs_url) = match binary_name {
            "claude" => (
                "Claude Code",
                "npm install -g @anthropic-ai/claude-code",
                "https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/overview",
            ),
            "agy" => (
                "Antigravity",
                "npm install -g @google/antigravity-cli",
                "https://idx.dev/antigravity",
            ),
            "gemini" => (
                "Gemini CLI",
                "npm install -g @google/gemini-cli",
                "https://github.com/google-gemini/gemini-cli",
            ),
            "codex" => (
                "OpenAI Codex CLI",
                "npm install -g @openai/codex",
                "https://github.com/openai/codex",
            ),
            "opencode" => (
                "OpenCode",
                "curl -fsSL https://opencode.ai/install | bash",
                "https://opencode.ai/docs",
            ),
            "aider" => (
                "Aider",
                "pipx install aider-chat",
                "https://aider.chat/docs/install.html",
            ),
            "cursor-agent" => (
                "Cursor CLI",
                "cursor --install-agent-cli",
                "https://docs.cursor.com/cli",
            ),
            "grok" => (
                "Grok CLI",
                "npm install -g @xai/grok-cli",
                "https://docs.x.ai",
            ),
            "q" => (
                "Amazon Q Developer CLI",
                "brew install amazon-q",
                "https://aws.amazon.com/q/developer/",
            ),
            "pi" => (
                "Pi Coding Agent",
                "npm install -g @mariozechner/pi-coding-agent",
                "https://github.com/badlogic/pi-mono",
            ),
            "omp" => (
                "Oh My Pi",
                "npm install -g oh-my-pi",
                "https://github.com/oh-my-pi/omp",
            ),
            "cmd" | "command-code" => (
                "Command Code",
                "npm install -g command-code",
                "https://commandcode.ai/docs",
            ),
            "hermes" => (
                "Hermes Agent",
                "cargo install hermes-cli",
                "https://github.com/hermes-agent/hermes",
            ),
            "muse" => (
                "Muse Code",
                "npm install -g @muse/cli",
                "https://musecode.dev/docs",
            ),
            other => (
                other,
                "Please install the CLI binary and ensure it is on your PATH",
                "https://github.com/agmux/agmux/blob/main/docs/agents.md",
            ),
        };
        MissingCliGuidance {
            binary_name: binary_name.to_string(),
            display_name: display_name.to_string(),
            install_command: install_command.to_string(),
            docs_url: docs_url.to_string(),
        }
    }
}

fn find_executable_in_dir(dir: &Path, binary: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates: Vec<PathBuf> = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into())
        .to_string_lossy()
        .split(';')
        .map(|ext| dir.join(format!("{binary}{ext}")))
        .collect();
    #[cfg(not(windows))]
    let candidates = [dir.join(binary)];
    candidates.into_iter().find(|path| {
        let Ok(metadata) = path.metadata() else {
            return false;
        };
        if !metadata.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    })
}
