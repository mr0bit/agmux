# Daemon startup diagnostics

The existing `daemon_spawner` module starts the sibling backend executable when
IPC is unavailable. It forwards the selected endpoint and token-file path and
retains the detached-process behavior. Startup stderr is written to a diagnostic
file beside the token: `<token-path>.daemon.log`. The file is rotated on the next
spawn attempt, is created with mode 0600 on Unix, and remains available after an
exit or readiness timeout. The daemon's stderr goes to a file rather than an
undrained pipe, so large output cannot stall startup.

Failure messages explain that the background service could not start, include
the exit status and up to the last 8 KiB of backend diagnostics, identify the log
path, and suggest `agmux doctor`. If no diagnostic is written, the message explains
how to check the paired executables. Launch/diagnostic-file permission failures
retain their OS error context. Readiness timeout remains three seconds and
explicitly notes that the backend may still be starting; it is not killed.
No token contents are deliberately printed. Logs can contain local paths and
backend diagnostic text and should be reviewed before sharing.

Verification uses the actual CLI and backend with a deliberately invalid database
path and checks the reason and retained log. A separate failing backend emits over
100 KiB to prove that stderr cannot fill a pipe and that displayed text is bounded.
Windows release CI repeats a real database failure and checks the readable cause.
