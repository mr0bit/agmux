# Agent context low-level design

## LLD-ORCH-AGENT-CONTEXT-006

The CLI handles `agent-context` before daemon startup. The overview is a compact static discovery index. Detailed references obtain usage directly from the clap command definition and append examples, including JSON output for task and lock. Unknown names return an error. This path has no IPC or persistent side effects. The CLI integration test exercises the public executable without a daemon.
