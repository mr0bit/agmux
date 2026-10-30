# Low-Level Design: Missing CLI Binary Detector & Guidance (`cli_detector`)

* **LLD ID:** `LLD-PLUGIN-MISSING-DETECTOR-006`
* **Upstream Requirement:** `REQ-PLUGIN-MISSING-DETECTOR-006`
* **Module:** `crates/agmux-core::cli_detector`
* **Verification Test:** `TEST-VER-PLUGIN-006`

## 1. Design Overview
`cli_detector` verifies whether a selected agent binary is present on `PATH` prior to PTY allocation and returns a structured `MissingCliGuidance` banner containing the official installation command and documentation URL when missing.
