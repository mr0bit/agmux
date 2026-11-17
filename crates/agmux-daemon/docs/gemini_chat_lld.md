
# Gemini conversation design — issue #17

## LLD-UX-CHAT-MESSAGES-105 / LLD-UX-TURN-LIFECYCLE-106
The daemon supervises one lazy `gemini --acp` subprocess for each Gemini session.
ACP uses JSON-RPC lines on stdio: initialize v1, session/new, session/prompt,
session/update and session/cancel. The official Windows npm launcher resolution
is reused; no shell-assembled command is executed. `GEMINI_CLI_NO_RELAUNCH=1`
keeps the directly supervised process responsible for ACP. Credentials stay in
the CLI's local settings/environment. No browser credential fields are added.
The client advertises no filesystem/terminal proxy capabilities. Native Gemini
tools remain under the provider's normal approval policy; approval requests are
queued and surfaced with the exact provider options, never auto-approved.

The native PTY and ACP are **independent conversation contexts**, sharing the
project and configured model. The UI explains this. ACP JSON never enters the
PTY stream; native TUI banners never become assistant messages.

States: idle → working → idle; permission suspends working; Stop requests
cancellation; cancelled allows another turn on a healthy ACP session. Failures,
startup errors and timeouts become failed with guidance to start a new session.
Initialization and session creation each have a 45-second deadline; a turn has a
300-second deadline including approval wait. Stop kills an unresponsive provider
after five seconds. Ending a session and graceful daemon shutdown reap ACP too.
The bridge returns immediately after accepting a turn. Browser reload/switching
recovers daemon-held messages; daemon restart does not persist ACP conversations.
The first phase intentionally does not claim durable provider-session recovery.

## LLD-UX-CHAT-ISOLATION-111
A bounded session map owns transports and histories. Per-session state lock
atomically validates idle state, deduplicates the supplied request ID and records
the user message before spawning work. Repeated accepted IDs return the same
state without a second provider prompt. The browser reuses an ID after an
ambiguous HTTP failure. Concurrent new requests are rejected, never silently
queued. Session-scoped ACP IDs filter incoming updates. Histories cap text bytes
at 2 MiB, user turns at under 190 message records, and open conversations at 32;
frames cap at 2 MiB. Browser snapshots contain lightweight statuses for all
conversations so the sidebar updates even outside Chat. Full transcript polling
is limited to the open conversation.

## LLD-UX-PROVIDER-MODELS-108
A separate short-lived ACP session discovers `models.availableModels` and
`currentModelId`; it does not send a model prompt. Successful inventories cache
by canonical project directory for 60 seconds, at most 32 entries; at most two
probes run concurrently. The probe is terminated after discovery/failure. Missing
authentication or unsupported inventories produce an honest configured-default
fallback. CLI-advertised inventory is not a promise of account quota/entitlement.
The chosen model is passed once to the native process and stored in the daemon
conversation configuration for ACP. Advanced native CLI options are not blindly
forwarded into the structured adapter.

## Verification seams
`TEST-VER-UX-105/106/111` exercise real HTTP + IPC + daemon + external ACP fixture:
ordered chunks, two turns, multiline, idempotency, rejected concurrency, separate
sessions/reload, explicit permission rejection, Stop and provider exit.
`TEST-VER-UX-108` probes the provider boundary and checks the advertised default.
Fixtures cover the protocol contract, not authenticated Gemini/model quality.
Authenticated two-turn native Windows evidence is still required for acceptance.

Windows ACP project paths are simplified with dunce before both process launch
and session/new serialization. The provider fixture rejects Windows verbatim
project paths, guarding Node module resolution alongside the existing launcher
entry-path regression.
