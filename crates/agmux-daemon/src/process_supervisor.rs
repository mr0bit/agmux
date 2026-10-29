//! Dedicated operating system process group supervisor.

/// Isolate a child process into its own operating system process group.
#[cfg(unix)]
pub fn isolate_process_group(pid: u32) -> Result<(), std::io::Error> {
    // Safety: libc::setpgid with pid and pgid=0 sets the process group ID to match its PID.
    let res = unsafe { libc::setpgid(pid as libc::pid_t, 0) };
    if res != 0 {
        let err = std::io::Error::last_os_error();
        // EACCES or EPERM can happen if child already called exec; not fatal if already leader
        if err.raw_os_error() != Some(libc::EACCES) && err.raw_os_error() != Some(libc::EPERM) {
            return Err(err);
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn isolate_process_group(_pid: u32) -> Result<(), std::io::Error> {
    Ok(())
}

/// Send a signal to an entire process group.
#[cfg(unix)]
pub fn kill_process_group(pgid: u32, signal: i32) -> Result<(), std::io::Error> {
    // Safety: passing negative pgid to kill delivers signal to entire process group
    let res = unsafe { libc::kill(-(pgid as libc::pid_t), signal) };
    if res != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn kill_process_group(_pgid: u32, _signal: i32) -> Result<(), std::io::Error> {
    Ok(())
}
