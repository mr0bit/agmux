# Low-Level Design: Network Cookie Token Authenticator (`network_auth`)

* **LLD ID:** `LLD-REMOTE-AUTH-TOKEN-003`
* **Upstream Requirement:** `REQ-REMOTE-AUTH-TOKEN-003`
* **Module:** `crates/agmux-daemon::network_auth`
* **Verification Test:** `TEST-VER-REMOTE-003`

## 1. Design Overview
`NetworkAuthValidator` enforces constant-time 256-bit cookie token validation (`daemon.token`) on all incoming IPC and network/tunnel connections, immediately rejecting unauthenticated frames and recording unauthorized attempt diagnostics.
