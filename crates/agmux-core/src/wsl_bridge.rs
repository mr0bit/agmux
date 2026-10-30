//! Windows Subsystem for Linux (WSL2) command and path bridge.

/// Translates Windows paths and builds `wsl.exe -d <distro> -- <command>` invocations.
#[derive(Debug, Clone, Default)]
pub struct WslBridge;

impl WslBridge {
    /// Convert a Windows path (e.g. `C:\Users\dev\repo` or `D:/work/app`) to a WSL2 mount path
    /// (`/mnt/c/Users/dev/repo` or `/mnt/d/work/app`). Paths already starting with `/` are returned unchanged.
    pub fn windows_to_wsl_path(win_path: &str) -> String {
        let trimmed = win_path.trim();
        if trimmed.starts_with('/') {
            return trimmed.to_string();
        }
        let normalized = trimmed.replace('\\', "/");
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            let drive = (bytes[0] as char).to_ascii_lowercase();
            let rest = normalized[2..].trim_start_matches('/');
            if rest.is_empty() {
                format!("/mnt/{drive}")
            } else {
                format!("/mnt/{drive}/{rest}")
            }
        } else {
            normalized
        }
    }

    /// Construct a `wsl.exe` command and argument vector for spawning a command inside `distro`.
    pub fn build_command(
        distro: &str,
        command: &str,
        args: &[String],
        windows_cwd: Option<&str>,
    ) -> (String, Vec<String>) {
        let mut wsl_args = vec!["-d".to_string(), distro.to_string()];
        if let Some(cwd) = windows_cwd.filter(|s| !s.trim().is_empty()) {
            wsl_args.push("--cd".to_string());
            wsl_args.push(Self::windows_to_wsl_path(cwd));
        }
        wsl_args.push("--".to_string());
        wsl_args.push(command.to_string());
        wsl_args.extend_from_slice(args);
        ("wsl.exe".to_string(), wsl_args)
    }
}
