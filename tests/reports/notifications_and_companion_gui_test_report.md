# Notifications and companion bridge verification

| Test | Requirement | Evidence |
| --- | --- | --- |
| `unicode_quote_is_last_120_characters` | REQ-GUI-NOTIFICATION-HOOKS-001 | Exercises the final 120 Unicode characters. |
| `hook_receives_all_fields` | REQ-GUI-NOTIFICATION-HOOKS-001 | Executes a real hook and checks all eight environment variables. |
| `hook_times_out_after_fifteen_seconds` | REQ-GUI-NOTIFICATION-HOOKS-001 | Waits for the actual 15-second timeout and checks that its descendant is no longer running. |
| `live_notifications_panels_http_reply_and_restart` | TEST-VER-GUI-001, TEST-VER-GUI-002 | Starts a daemon and HTTP companion; verifies a Tier 1 notification and output, registered file and web panels, deep-link validation, origin rejection, file editing, inline PTY reply, and persisted panel selection and notifications after daemon restart. |

The focused end-to-end test passed on Linux. Full workspace suite (`cargo test --workspace`) passed with 85 tests passing and zero failures.
Live terminal verification screenshot: [task-32_verification.png](../../docs/assets/screenshots/task-32_verification.png).
