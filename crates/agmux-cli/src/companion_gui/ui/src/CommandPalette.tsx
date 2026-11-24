import { useEffect, useRef, useState } from "react";
export type WorkspaceCommand = {
  id: string;
  label: string;
  detail?: string;
  shortcut?: string;
  run: () => void;
};
export default function CommandPalette({
  mode,
  commands,
  onClose,
}: {
  mode: "commands" | "sessions" | "shortcuts";
  commands: WorkspaceCommand[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState(""),
    [index, setIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null),
    host = useRef<HTMLDivElement>(null);
  const rows = commands
    .filter((c) =>
      (c.label + " " + (c.detail || ""))
        .toLowerCase()
        .includes(query.toLowerCase()),
    )
    .slice(0, 60);
  useEffect(() => {
    input.current?.focus();
  }, []);
  useEffect(() => {
    setQuery("");
    setIndex(0);
    input.current?.focus();
  }, [mode]);
  useEffect(() => setIndex(0), [query]);
  const run = (command: WorkspaceCommand) => {
    onClose();
    command.run();
  };
  return (
    <div
      className="palette-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={host}
        className="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label={
          mode === "sessions"
            ? "Quick session switcher"
            : mode === "shortcuts"
              ? "Keyboard shortcuts"
              : "Command palette"
        }
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onClose();
          }
          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            setIndex((i) =>
              rows.length
                ? (i + (e.key === "ArrowDown" ? 1 : -1) + rows.length) %
                  rows.length
                : 0,
            );
          }
          if (e.key === "Enter" && e.target === input.current && rows[index]) {
            e.preventDefault();
            run(rows[index]);
          }
          if (e.key === "Tab") {
            const controls = [
              ...(host.current?.querySelectorAll<HTMLElement>("button,input") ||
                []),
            ].filter((el) => el.tabIndex >= 0);
            const first = controls[0],
              last = controls.at(-1);
            if (e.shiftKey && document.activeElement === first) {
              e.preventDefault();
              last?.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
              e.preventDefault();
              first?.focus();
            }
          }
        }}
      >
        <div className="palette-search">
          <input
            ref={input}
            aria-label={
              mode === "sessions"
                ? "Find a session or project"
                : "Find a command"
            }
            role="combobox"
            aria-expanded="true"
            aria-controls="workspace-palette-options"
            aria-activedescendant={rows[index] ? `palette-${index}` : undefined}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={
              mode === "sessions"
                ? "Search sessions and project paths…"
                : "> Type a command…"
            }
          />
          <button
            className="icon-button"
            aria-label="Close command palette"
            onClick={onClose}
          >
            Esc
          </button>
        </div>
        <div
          id="workspace-palette-options"
          role="listbox"
          className="palette-options"
        >
          {rows.map((command, i) => (
            <button
              id={`palette-${i}`}
              role="option"
              aria-selected={index === i}
              tabIndex={-1}
              key={command.id}
              onMouseEnter={() => setIndex(i)}
              onClick={() => run(command)}
            >
              <span>
                <strong>{command.label}</strong>
                {command.detail && <small>{command.detail}</small>}
              </span>
              {command.shortcut && <kbd>{command.shortcut}</kbd>}
            </button>
          ))}
          {!rows.length && (
            <p className="live-hint">No matching commands or sessions.</p>
          )}
        </div>
        <footer>↑ ↓ navigate · Enter select · Esc close</footer>
      </div>
    </div>
  );
}
