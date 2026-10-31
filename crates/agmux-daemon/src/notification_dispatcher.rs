//! Bounded execution of notification hooks away from the PTY event loop.
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Metadata supplied to a notification hook as environment variables.
#[derive(Clone, Debug)]
pub struct HookNotification {
    pub kind: String,
    pub session_id: String,
    pub session_name: String,
    pub tool: String,
    pub cwd: String,
    pub branch: String,
    pub title: String,
    pub body: String,
}

/// Return the last 120 Unicode scalar values from output.
pub fn latest_quote(output: &str) -> String {
    let mut chars: Vec<char> = output.chars().rev().take(120).collect();
    chars.reverse();
    chars.into_iter().collect()
}

/// Read a Git worktree's current branch without starting a subprocess.
pub fn workspace_branch(cwd: &str) -> String {
    for dir in std::path::Path::new(cwd).ancestors() {
        let dotgit = dir.join(".git");
        let gitdir = if dotgit.is_dir() {
            dotgit
        } else if let Ok(pointer) = std::fs::read_to_string(&dotgit) {
            match pointer.trim().strip_prefix("gitdir: ") {
                Some(path) => dir.join(path),
                None => continue,
            }
        } else {
            continue;
        };
        if let Ok(head) = std::fs::read_to_string(gitdir.join("HEAD")) {
            return head
                .trim()
                .strip_prefix("ref: refs/heads/")
                .unwrap_or("")
                .to_owned();
        }
    }
    String::new()
}

/// A dispatcher drops work when its bounded execution capacity is exhausted.
#[derive(Clone)]
pub struct NotificationDispatcher {
    slots: Arc<Semaphore>,
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(4)),
        }
    }
}

impl NotificationDispatcher {
    /// Reserve bounded capacity before gathering metadata or starting a hook.
    pub fn try_reserve(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.slots.clone().try_acquire_owned().ok()
    }

    /// Schedule a configured shell hook without blocking the caller.
    pub fn dispatch(&self, hook: String, notification: HookNotification) {
        let Ok(permit) = self.slots.clone().try_acquire_owned() else {
            tracing::warn!("notification hook capacity exhausted");
            return;
        };
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = run_hook(&hook, &notification).await {
                tracing::warn!(%error, "notification hook failed");
            }
        });
    }
}

#[cfg(windows)]
struct HookJob(usize);
#[cfg(windows)]
impl HookJob {
    fn attach(child: &tokio::process::Child) -> std::io::Result<Self> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let handle = child
                .raw_handle()
                .ok_or_else(|| std::io::Error::other("hook process exited"));
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) != 0
                && handle
                    .map(|h| AssignProcessToJobObject(job, h as _) != 0)
                    .unwrap_or(false);
            if !ok {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(error);
            }
            Ok(Self(job as usize))
        }
    }
}
#[cfg(windows)]
impl Drop for HookJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}

/// Run the configured shell command with session data passed only via environment.
pub async fn run_hook(hook: &str, n: &HookNotification) -> std::io::Result<()> {
    #[cfg(unix)]
    let mut command = tokio::process::Command::new("sh");
    #[cfg(windows)]
    let mut command = tokio::process::Command::new("cmd");
    #[cfg(unix)]
    command.arg("-c").arg(hook).process_group(0);
    #[cfg(windows)]
    command.arg("/C").arg(hook);
    command
        .env("AGMUX_NOTIFY_KIND", &n.kind)
        .env("AGMUX_SESSION_ID", &n.session_id)
        .env("AGMUX_SESSION_NAME", &n.session_name)
        .env("AGMUX_TOOL", &n.tool)
        .env("AGMUX_CWD", &n.cwd)
        .env("AGMUX_BRANCH", &n.branch)
        .env("AGMUX_TITLE", &n.title)
        .env("AGMUX_BODY", &n.body)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn()?;
    #[cfg(unix)]
    let pid = child.id();
    #[cfg(windows)]
    let job = match HookJob::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    match tokio::time::timeout(Duration::from_secs(15), child.wait()).await {
        Ok(result) => {
            // A hook may have backgrounded children before its shell exited.
            #[cfg(unix)]
            if let Some(pid) = pid {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            result?;
        }
        Err(_) => {
            #[cfg(unix)]
            if let Some(pid) = pid {
                // The child is the leader of its own process group.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            #[cfg(windows)]
            drop(job);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "notification hook timed out",
            ));
        }
    }
    Ok(())
}
