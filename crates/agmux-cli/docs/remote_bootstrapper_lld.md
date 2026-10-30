# Low-Level Design: Pinned Remote Binary Bootstrapper (`remote_bootstrapper`)

* **LLD ID:** `LLD-REMOTE-BOOTSTRAP-PINNED-002`
* **Upstream Requirement:** `REQ-REMOTE-BOOTSTRAP-PINNED-002`
* **Module:** `crates/agmux-cli::remote_bootstrapper`
* **Verification Test:** `TEST-VER-REMOTE-002`

## 1. Design Overview
`RemoteBootstrapper` parses remote `uname -s -m` output into a canonical Rust target triple (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`), verifies downloaded release binary bytes using a pure-Rust FIPS 180-4 SHA-256 implementation against the pinned digest, and installs the verified executable to `~/.local/bin/agmuxd` with `0755` permissions.
