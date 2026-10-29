# Viewport renderer

## LLD-TUI-RATATUI-VIEWPORT-001

The client authenticates, lists sessions, and subscribes to the selected session. The daemon sends the captured terminal contents followed by PTY output frames; changing selection replaces the subscription. A VT100 parser maintains the displayed text grid. State events update sidebar badges. Ratatui renders the sidebar, viewport, and footer, and the terminal guard restores terminal settings on exit or error.

This tracer does not yet implement the hierarchy, full color rendition, attachment, or memory verification required by the upstream specification.
