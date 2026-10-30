# MCP server low-level design

## LLD-ORCH-MCP-SERVER-007

The stdio adapter reads one JSON-RPC message per line and writes only JSON-RPC responses to stdout; diagnostics go to stderr. It handles initialization, tool discovery, calls, ping and notifications. Parse, request, method and argument errors use JSON-RPC error codes; daemon failures are MCP tool errors. Tool schemas describe the sixteen supported operations. Each call creates an authenticated framed daemon connection; the adapter never accesses the database. `read_session` uses an IPC snapshot request for active or persisted sessions. File paths use the same canonicalization as the CLI. `agmux mcp` and `agmux --enable-mcp` start the server. `agmux run --enable-mcp` registers it for Claude or Codex using that agent's command-line configuration; other agents can configure `agmux mcp` directly.
