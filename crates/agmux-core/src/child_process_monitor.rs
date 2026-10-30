//! Tier 2 child process tree inspection filtering out Zombie (`Z`) and Dead (`X`) states.

/// Inspects OS child processes of a session leader PID to determine whether subcommands are active.
#[derive(Debug, Clone, Default)]
pub struct ChildProcessMonitor;

impl ChildProcessMonitor {
    /// Return all live (non-Zombie `'Z'`, non-Dead `'X'`) direct child PIDs of `parent_pid`.
    pub fn active_child_pids(parent_pid: u32) -> Vec<u32> {
        #[cfg(target_os = "linux")]
        {
            let mut children = Vec::new();
            let task_dir = format!("/proc/{parent_pid}/task");
            if let Ok(entries) = std::fs::read_dir(&task_dir) {
                for entry in entries.flatten() {
                    let children_path = entry.path().join("children");
                    if let Ok(content) = std::fs::read_to_string(children_path) {
                        for tok in content.split_whitespace() {
                            if let Ok(cpid) = tok.parse::<u32>()
                                && Self::is_process_alive_non_zombie(cpid)
                                && !children.contains(&cpid)
                            {
                                children.push(cpid);
                            }
                        }
                    }
                }
            }
            if !children.is_empty() {
                return children;
            }
            // Fallback to scanning /proc/*/stat for PPID == parent_pid if /proc/<pid>/task/*/children is empty
            if let Ok(proc_entries) = std::fs::read_dir("/proc") {
                for entry in proc_entries.flatten() {
                    if let Some(name) = entry.file_name().to_str()
                        && let Ok(pid) = name.parse::<u32>()
                        && let Some((state, ppid)) = Self::read_proc_stat_state_and_ppid(pid)
                        && ppid == parent_pid
                        && !matches!(state, 'Z' | 'X')
                        && !children.contains(&pid)
                    {
                        children.push(pid);
                    }
                }
            }
            children
        }

        #[cfg(all(unix, not(target_os = "linux")))]
        {
            let output = std::process::Command::new("pgrep")
                .args(["-P", &parent_pid.to_string()])
                .output();
            match output {
                Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                    .split_whitespace()
                    .filter_map(|s| s.parse::<u32>().ok())
                    .collect(),
                _ => Vec::new(),
            }
        }

        #[cfg(windows)]
        {
            let _ = parent_pid;
            Vec::new()
        }
    }

    /// Returns `true` if `parent_pid` has at least one live non-zombie child process.
    pub fn has_active_children(parent_pid: u32) -> bool {
        !Self::active_child_pids(parent_pid).is_empty()
    }

    /// Check `/proc/<pid>/stat` on Linux to verify the process exists and is not Zombie (`'Z'`) or Dead (`'X'`).
    #[cfg(target_os = "linux")]
    pub fn is_process_alive_non_zombie(pid: u32) -> bool {
        match Self::read_proc_stat_state_and_ppid(pid) {
            Some((state, _)) => !matches!(state, 'Z' | 'X'),
            None => false,
        }
    }

    #[cfg(target_os = "linux")]
    fn read_proc_stat_state_and_ppid(pid: u32) -> Option<(char, u32)> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let close_paren = stat.rfind(')')?;
        let after = stat.get(close_paren + 1..)?.trim_start();
        let mut parts = after.split_whitespace();
        let state = parts.next()?.chars().next()?;
        let ppid = parts.next()?.parse::<u32>().ok()?;
        Some((state, ppid))
    }
}
