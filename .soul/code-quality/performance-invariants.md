# Code Quality: Performance Invariants & Resource Budgets

## 1. Overview
High-performance software targets ultra-low overhead, bounded memory consumption, and zero CPU waste, allowing background services and developer tools to run continuously without degrading host machine responsiveness.

---

## 2. Universal Non-Negotiable Invariants

1. **Zero Unneeded Heap Allocations in Hot Loops:**
   * High-throughput byte stream ingestion and parsing.
   * Frame diff rendering and dirty state delta computation.
   * Search indexing, tokenization, and regex evaluation.
   * Event stream broadcasting.
   * *Rule:* Reuse allocated buffers, pass borrowed slices, and avoid allocating temporary strings or collections inside recurring execution loops.
2. **Idle CPU Usage SLA (Zero Waste at Rest):**
   * When all managed background processes or workers are in rest states (idle, waiting, stopped), system CPU usage must measure **0.0%**.
   * Active polling loops are strictly prohibited; rely entirely on event-driven async reactors (epoll, kqueue, IOCP) and silence timers.
3. **Dead-Process Zero-RAM Footprint:**
   * When a managed child process or sub-entity is terminated, immediately release its in-memory parsers, caches, and file descriptors. Active RAM consumption for dead entities must drop to **0 bytes**.
