import { useEffect, useRef, useState } from "react";
export type WorkspaceCommand = {
  id: string;
  label: string;
  detail?: string;
  shortcut?: string;
  run: () => void;
};

function matchRank(label: string, detail: string, query: string) {
  if (label.startsWith(query)) return 0;
  if (label.includes(query)) return 1;
  if (detail.includes(query)) return 2;
  let matched = 0;
  for (const character of label) {
    if (character === query[matched]) matched++;
    if (matched === query.length) return 3;
  }
  return Infinity;
}

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
  const search = (mode === "commands" ? query.replace(/^\s*>/, "") : query)
    .trim()
    .toLowerCase();
  const rows = commands
    .map((command) => {
      const label = command.label.toLowerCase();
      const detail = command.detail?.toLowerCase() ?? "";
      const rank =
        mode === "commands"
          ? matchRank(label, detail, search)
          : (`${label} ${detail}`).includes(search)
            ? 0
            : Infinity;
      return { command, rank };
    })
    .filter(({ rank }) => Number.isFinite(rank))
    .sort((a, b) => a.rank - b.rank)
    .slice(0, 60)
    .map(({ command }) => command);
  const activeIndex = Math.min(index, Math.max(rows.length - 1, 0));
  useEffect(() => {
    setQuery("");
    setIndex(0);
    input.current?.focus();
  }, [mode]);
  useEffect(() => {
    host.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({
      block: "nearest",
    });
  }, [activeIndex, search]);
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
            setIndex(
              rows.length
                ? (activeIndex + (e.key === "ArrowDown" ? 1 : -1) + rows.length) %
                  rows.length
                : 0,
            );
          }
          if (
            e.key === "Enter" &&
            !e.nativeEvent.isComposing &&
            e.target === input.current &&
            rows[activeIndex]
          ) {
            e.preventDefault();
            run(rows[activeIndex]);
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
          {mode === "commands" && (
            <span className="palette-prompt" aria-hidden="true">
              &gt;
            </span>
          )}
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
            aria-activedescendant={
              rows[activeIndex] ? `palette-${activeIndex}` : undefined
            }
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setIndex(0);
            }}
            placeholder={
              mode === "sessions"
                ? "Search sessions and project paths…"
                : "Type a command…"
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
              aria-selected={activeIndex === i}
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
            <p className="live-hint" role="status">
              {mode === "sessions"
                ? "No matching sessions."
                : "No matching commands."}
            </p>
          )}
        </div>
        <footer>↑ ↓ navigate · Enter select · Esc close</footer>
      </div>
    </div>
  );
}
