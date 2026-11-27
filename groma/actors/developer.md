---
type: C4 Actor
title: Developer
status: stable
groma:
  id: developer
---

Runs and monitors coding agents, switches between sessions, and reviews work from the terminal or companion browser application.

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [developer](developer.md) | [agmux-agmux](../systems/agmux/containers/agmux-agmux/container.md) | Manages agent sessions | CLI and terminal UI |
| [developer](developer.md) | [agmux-ui](../systems/agmux/containers/agmux-ui/container.md) | Monitors agent sessions | Web browser |
