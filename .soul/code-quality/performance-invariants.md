# Code Quality: Performance Invariants & Memory Budgets

## 1. Overview
Agmux targets ultra-lightweight execution (~10 MB RAM daemon, ~5 MB binary) capable of running continuously on developer machines without battery or CPU drain.

---

## 2. Non-Negotiable Invariants

1. **Zero Unneeded Heap Allocations in Hot Loops:**
   * PTY byte burst ingestion (reading 8 KB chunks from master PTY).
   * VT100 dirty cell delta computation.
   * Aho-Corasick terminal tail regex scanning.
   * Event stream dispatch.
   * *Rule:* Reuse buffers, pass slices (`&[u8]`, `&str`), and avoid allocating temporary `String` or `Vec` objects on every tick.
2. **Idle CPU Usage SLA:**
   * When all managed agent sessions are in rest states (`IDLE`, `WAITING`, `DEAD`), `agmuxd` CPU usage must measure **0.0%**.
   * Polling loops are strictly prohibited; rely entirely on event-driven async reactors (epoll, kqueue, IOCP) and silence timers.
3. **Dead-Session Zero-RAM Footprint:**
   * When an agent session is terminated (`x` -> `DEAD`), drop the `vt100::Parser` struct and close all PTY file descriptors immediately. Active RAM consumption for that session must drop to **0 bytes**.
