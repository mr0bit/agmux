# Implementation Brief: Task 33 - Binary Self-Updater Subsystem (agmux update)

## 1. Context & Objective
You are the delegated implementer for **agmux** working on **TASK-33** (`REQ-GUI-SELF-UPDATER-003`).
Agmux is an ultra-fast, lightweight cross-platform agentic terminal multiplexer written in modern Rust (2024 Edition, MSRV 1.85+).
Your goal is to implement the self-updater subsystem in `crates/agmux-cli` enabling users to run `agmux update` (and `agmux --update`), safely checking for new GitHub releases, verifying cryptographic SHA-256 checksums, and atomically replacing `agmux` and `agmuxd` binaries with rollback protection.

## 2. Scope & Target Files
### New Files:
- `crates/agmux-cli/src/self_updater.rs`: Core self-updater module.
- `tests/e2e_self_updater_test.rs`: Level 5 SVT verification test suite.

### Modified Files:
- `crates/agmux-cli/Cargo.toml`: You are explicitly authorized to add any needed dependencies here.
- `Cargo.lock`: Updated cargo lockfile.
- `crates/agmux-cli/src/lib.rs`: Expose `pub mod self_updater;`.
- `crates/agmux-cli/src/main.rs`: Integrate `Commands::Update` and `--update` CLI flag.
- `docs/specs/11_desktop_gui_and_notifications_spec.md`: Update `REQ-GUI-SELF-UPDATER-003` to `Implementation Status: Verified` and `Test Status: Passed`.
- `.backlog/tasks/task-33 - Tracer-Bullet-Binary-Self-Updater-Checksum-Atomic-Replacement.md`: Mark acceptance criteria `[x]` and `status: Done`.

Note: You can reuse the pure-Rust FIPS 180-4 SHA-256 implementation in `crates/agmux-cli/src/remote_bootstrapper.rs` (`RemoteBootstrapper::sha256_hex`), or add dependencies in `crates/agmux-cli/Cargo.toml`. Both `crates/agmux-cli/Cargo.toml` and `Cargo.lock` are fully authorized for edits.

Do NOT perform speculative refactors outside of this scope.

## 3. Detailed Specifications (`REQ-GUI-SELF-UPDATER-003`)
1. **CLI Commands & Flags:**
   - Subcommand: `agmux update [--check] [--force] [--version <target>]`
   - Top-level flag alias: `agmux --update` (maps to `Commands::Update { check: false, force: false, version: None }`).
   - If `--check` is specified, check GitHub Releases and print whether an update is available (e.g. `Current: v0.1.0, Latest: v0.2.0 - Update available!`) without modifying files, exiting 0.
2. **Release Discovery & Version Comparison:**
   - Fetch latest release metadata from GitHub Releases API: `https://api.github.com/repos/mr0bit/agmux/releases/latest` (allow overriding base URL via `AGMUX_UPDATE_URL` for testing/custom mirrors).
   - Parse tag name (e.g. `v0.2.0` -> `0.2.0`). Compare semver against `env!("CARGO_PKG_VERSION")`.
   - If current version >= latest and not `--force`, report up to date and return Ok.
3. **Target Triple & Asset Identification:**
   - Determine current target triple (e.g. `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`).
   - Find matching asset: `agmux-v{version}-{target}.tar.gz` (or `.zip` on Windows).
   - Find checksum asset: `SHA256SUMS` or `SHA256SUMS.txt`.
4. **Download & Cryptographic Verification:**
   - Download the target archive into a temporary directory.
   - Compute SHA-256 hash of the downloaded archive.
   - Compare computed hash against the hash listed in `SHA256SUMS` for that asset. If hash does not match, abort with error and delete the temp file.
5. **Atomic Binary Replacement:**
   - Extract the `agmux` and `agmuxd` binaries from the archive.
   - Locate current running executable path (`std::env::current_exe()`).
   - Locate `agmuxd` in the same directory (or fallback to `~/.local/bin/agmuxd`).
   - Atomically replace the existing binaries:
     - On Unix: write new binary to `<dest>.tmp.<pid>`, set permissions to `0o755`, rename over `<dest>`. Keep `<dest>.old` backup to rollback if any step fails.
     - On Windows: rename `<dest>` to `<dest>.old`, write new binary.
   - Print success confirmation with the new version.
6. **E2E Integration Test (`tests/e2e_self_updater_test.rs`):**
   - Spins up a local test HTTP server (or uses a mock handler / `wiremock` / `tokio::net::TcpListener`) returning mock release JSON, mock `SHA256SUMS`, and a test tar.gz archive.
   - Tests:
     1. `test_self_updater_check_only_reports_available_update`: Verifies `--check` queries release and detects update without modifying files.
     2. `test_self_updater_checksum_mismatch_aborts`: Verifies corrupted archive download fails SHA-256 verification and preserves existing binaries.
     3. `test_self_updater_atomic_replacement_success`: Verifies valid archive and matching checksum replaces the binary file with new version content and maintains executable permissions.

## 4. Verification Gates
Before reporting completion, run and ensure 100% clean passes:
- `cargo check --workspace`
- `cargo test --test e2e_self_updater_test`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- `node .soul/tools/v-cycle-checker.mjs` (must pass with 0 errors)

## 5. Constraints
- Do NOT commit changes (`git commit`). The orchestrator will review the diff and land commits.
- Follow the Karpathy guidelines: keep code clean, simple, surgical, with robust error handling.
- Preserve all existing functionality and documentation.
