---
type: C4 Component
title: GeminiChat
status: stable
groma:
  id: geminichat
  parent: agmux-ui
  code:
    - scanner: react
      file: crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx
    - scanner: typescript
      file: crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx
      symbol: GeminiChat
---

## Derived relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx](../../../../../../crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx) | [crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx](../../../../../../crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx) | Invokes supplied callback: onDraft | react |
