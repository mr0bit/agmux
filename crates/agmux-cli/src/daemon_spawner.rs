//! Background daemon auto-spawner on socket absence.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use agmux_proto::connect_ipc;
use anyhow::{Context, Result, bail};

/// Locate the `agmuxd` daemon binary on the host system.
pub fn find_daemon_binary() -> Result<PathBuf> {
    if let Ok(bin) = std::env::var("AGMUXD_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Ok(p);
        }
    }

    let exe_name = format!("agmuxd{}", std::env::consts::EXE_SUFFIX);

    // Check directory of the currently running executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join(&exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // Check target/debug or target/release if developing locally
    let debug_fallback = format!("target/debug/{}", exe_name);
    let release_fallback = format!("target/release/{}", exe_name);
    for fallback in &[&debug_fallback, &release_fallback] {
        let p = PathBuf::from(fallback);
        if p.exists() {
            return Ok(p);
        }
    }

    // Fall back to searching PATH
    Ok(PathBuf::from(exe_name))
}

/// Ensure the background daemon process is alive, spawning it detached if inactive.
///
/// If the daemon socket is not responsive, this spawns `agmuxd` detached
/// and polls for readiness every 50ms up to 3 seconds.
pub async fn ensure_daemon_running(socket_path: &Path, token_path: &Path) -> Result<()> {
    if connect_ipc(socket_path).await.is_ok() {
        return Ok(());
    }

    let daemon_bin = find_daemon_binary()?;
    tracing::info!(
        "Daemon inactive on {}. Auto-spawning {}...",
        socket_path.display(),
        daemon_bin.display()
    );

    let mut cmd = std::process::Command::new(&daemon_bin);
    cmd.arg("--socket").arg(socket_path);
    cmd.arg("--token-path").arg(token_path);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    let mut log_name = token_path.as_os_str().to_os_string();
    log_name.push(".daemon.log");
    let log_path = PathBuf::from(log_name);
    if let Some(parent) = log_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("Agmux could not create its diagnostic directory {}. Check that your account can write there.", parent.display()))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let log = options.open(&log_path).with_context(|| {
        format!(
            "Agmux could not write its backend diagnostic log at {}. Check the folder permissions.",
            log_path.display()
        )
    })?;
    cmd.stderr(std::process::Stdio::from(log));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Keep the background daemon from opening a second console window.
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Agmux could not launch its background service at {}. Extract agmux and agmuxd from the same release archive into one folder. Startup log: {}",
            daemon_bin.display(), log_path.display()
        )
    })?;

    // Poll socket readiness with 50ms intervals up to 3 seconds (60 attempts)
    let max_attempts = 60;
    let poll_interval = Duration::from_millis(50);

    for attempt in 1..=max_attempts {
        tokio::time::sleep(poll_interval).await;

        if connect_ipc(socket_path).await.is_ok() {
            tracing::info!(
                "Daemon successfully auto-spawned and ready after {}ms",
                attempt * 50
            );
            return Ok(());
        }

        // Check if child exited prematurely
        if let Ok(Some(status)) = child.try_wait() {
            bail!(
                "Agmux could not start its background service ({}). The service stopped during startup ({status}).\n\n{}",
                daemon_bin.display(),
                startup_diagnostic(&log_path)
            );
        }
    }

    bail!(
        "Agmux's background service did not become ready within 3 seconds at {}. It may still be starting.\n\n{}",
        socket_path.display(),
        startup_diagnostic(&log_path)
    );
}

fn startup_diagnostic(log_path: &Path) -> String {
    let detail = (|| -> std::io::Result<String> {
        let mut file = std::fs::File::open(log_path)?;
        let length = file.metadata()?.len().min(8192);
        file.seek(SeekFrom::End(-(length as i64)))?;
        let mut bytes = Vec::with_capacity(length as usize);
        file.take(8192).read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).trim().to_owned())
    })();
    let reason = match detail {
        Ok(text) if !text.is_empty() => format!("Backend details:\n{text}"),
        Ok(_) => "The backend did not write an error message. Check that both executables came from the same release and were fully extracted.".into(),
        Err(error) => format!("The startup diagnostic could not be read: {error}"),
    };
    format!(
        "{reason}\n\nStartup log: {}\nRun `agmux doctor` to check local configuration.",
        log_path.display()
    )
}
