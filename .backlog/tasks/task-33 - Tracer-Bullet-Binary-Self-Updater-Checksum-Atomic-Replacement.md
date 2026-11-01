---
id: TASK-33
title: >-
  Tracer Bullet: Binary Self-Updater Subsystem (agmux update & SHA-256 Checksums)
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-01T06:15:00Z'
updated_date: '2026-11-01T06:15:00Z'
labels:
  - tracer-bullet
  - updater
  - phase-3
milestone: m-3
dependencies:
  - TASK-32
references:
  - docs/specs/11_desktop_gui_and_notifications_spec.md
priority: high
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements the Binary Self-Updater Subsystem (`FR-P3-03`):
1. `REQ-GUI-SELF-UPDATER-003` (`crates/agmux-cli::self_updater`):
   - Adds `agmux update` command and `--update` flag.
   - Queries GitHub Releases API for the latest release version and metadata.
   - Compares latest version against current `CARGO_PKG_VERSION` (`--check` / `--check-only` flag support).
   - Downloads target-triple binary archive for the host platform.
   - Validates SHA-256 checksums against the official release checksums asset (`SHA256SUMS`).
   - Performs atomic in-place executable replacement of `agmux` and `agmuxd` with rollback on failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `agmux update --check` checks GitHub Releases API and reports if a newer version is available
- [ ] #2 Asset downloader validates target-triple binary against SHA256SUMS and rejects corrupted/tampered payloads
- [ ] #3 Executable replacement replaces `agmux` and `agmuxd` atomically and verifies executable permissions
- [ ] #4 Integration test (`tests/e2e_self_updater_test.rs`) verifies mock release checking, checksum verification, and update lifecycle
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Dispatched to OpenAI Codex (Lane: feature1, gpt-6.1-terra).
<!-- SECTION:NOTES:END -->
