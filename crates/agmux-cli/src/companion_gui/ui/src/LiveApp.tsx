import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import {
  AnimatePresence,
  motion,
  MotionConfig,
  useReducedMotion,
} from "motion/react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  FileCode2,
  Folder,
  GitBranch,
  Menu,
  MessageSquare,
  Monitor,
  PanelLeft,
  PanelRight,
  Plus,
  Search,
  Settings,
  SquareTerminal,
  X,
  Zap,
} from "lucide-react";
import {
  Backend,
  type MachineInfo,
  type Metadata,
  type NativeDiff,
  type NativeSession,
  type Snapshot,
} from "./backend";
import type { Session } from "./demo";
import type { DiffLine } from "./diff-model";
import { Diff } from "./CodeViews";
import AgentIcon from "./AgentAvatar";
import Composer from "./Composer";
import EmptyState from "./EmptyState";
import { AnimatedCheck, AnimatedOverlay, Selection } from "./MotionUI";
import { motionTiming } from "./motion-spec";
import { PanelDivider, SessionList } from "./WorkspaceLists";
const LiveTerminal = lazy(() => import("./LiveTerminal"));
const CodeEditor = lazy(() => import("./CodeEditor"));
type Tab = "Chat" | "Terminal" | "Files" | "Git" | "Machines";
type Entry = { name: string; path: string; is_dir: boolean };
type OpenFile = {
  id: number;
  path: string;
  content: string;
  original: string;
  version: string;
};
function saved<T>(key: string, initial: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) || "null") ?? initial;
  } catch {
    return initial;
  }
}
export function normalizeSession(
  session: NativeSession,
  metadata?: Metadata,
): Session {
  const command =
    session.command.replaceAll("\\", "/").split("/").pop()?.split(" ")[0] ||
    "Shell";
  const agent =
    (
      {
        claude: "Claude Code",
        codex: "Codex",
        opencode: "OpenCode",
        gemini: "Gemini",
        "cursor-agent": "Cursor",
        sh: "Shell",
        bash: "Shell",
        zsh: "Shell",
        "cmd.exe": "Shell",
        pwsh: "Shell",
      } as Record<string, string>
    )[command] || command;
  return {
    id: session.session_id,
    title: metadata?.project || `${agent} · ${session.session_id.slice(-6)}`,
    project: session.cwd.replaceAll("\\", "/").split("/").pop() || session.cwd,
    agent,
    state: ["WAITING", "WAITING_INPUT", "STUCK", "FAILED", "ERRORED"].includes(
      session.state,
    )
      ? "attention"
      : ["WORKING", "RUNNING", "STARTING"].includes(session.state)
        ? "working"
        : ["DEAD", "COMPLETED", "EXITED"].includes(session.state)
          ? "stopped"
          : "idle",
    time: session.state.toLowerCase().replaceAll("_", " "),
    pinned: metadata?.favorite,
  };
}
function diffLines(file: NativeDiff): DiffLine[] {
  return file.hunks.flatMap((hunk) => [
    { kind: "hunk" as const, text: hunk.header },
    ...hunk.lines.map((line) => ({
      kind:
        line.kind === "Added"
          ? ("add" as const)
          : line.kind === "Removed"
            ? ("del" as const)
            : ("context" as const),
      text: line.content,
      old: line.old_line ?? undefined,
      next: line.new_line ?? undefined,
    })),
  ]);
}
const fallbackMetadata: Metadata = {
  project: "",
  group: "",
  favorite: false,
  archived: false,
};
export default function LiveApp({ token }: { token: string }) {
  const backend = useMemo(() => new Backend(token), [token]);
  const reduced = !!useReducedMotion();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null),
    [info, setInfo] = useState<MachineInfo | null>(null);
  const [selected, setSelected] = useState<string | undefined>(),
    [connected, setConnected] = useState(false),
    [retry, setRetry] = useState(0);
  const [tab, setTab] = useState<Tab>("Chat"),
    [search, setSearch] = useState(""),
    [sidebar, setSidebar] = useState(window.innerWidth >= 700),
    [review, setReview] = useState(window.innerWidth >= 1050),
    [reviewWidth, setReviewWidth] = useState(420);
  const [light, setLight] = useState(
      saved("agmux-appearance", { light: false }).light,
    ),
    [density, setDensity] = useState<"comfortable" | "compact">(
      saved("agmux-appearance", { density: "comfortable" }).density as
        | "comfortable"
        | "compact",
    );
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({}),
    [drafts, setDrafts] = useState<Record<string, string>>({});
  const [modal, setModal] = useState<"new" | "settings" | "send" | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const [newAgent, setNewAgent] = useState("shell"),
    [newTitle, setNewTitle] = useState(""),
    [cwd, setCwd] = useState(""),
    [custom, setCustom] = useState(""),
    [args, setArgs] = useState("[]"),
    [geminiModel, setGeminiModel] = useState("");
  const [directory, setDirectory] = useState(""),
    [entries, setEntries] = useState<Entry[]>([]),
    [editor, setEditor] = useState<OpenFile | null>(null),
    [filesLoading, setFilesLoading] = useState(false);
  const [diffs, setDiffs] = useState<NativeDiff[]>([]),
    [diffScope, setDiffScope] = useState("FullUncommitted"),
    [diffLayout, setDiffLayout] = useState<"unified" | "split">("unified"),
    [diffLoading, setDiffLoading] = useState(false);
  const [recipient, setRecipient] = useState(""),
    [outgoing, setOutgoing] = useState("");
  const chooseSequence = useRef(0),
    output = useRef<HTMLPreElement>(null),
    follow = useRef(true),
    fileSequence = useRef(0);
  const [readingBack, setReadingBack] = useState(false);
  const sessions = useMemo(
    () =>
      (snapshot?.sessions || []).map((item) =>
        normalizeSession(item, snapshot?.metadata[item.session_id]),
      ),
    [snapshot?.sessions, snapshot?.metadata],
  );
  const session =
    sessions.find((item) => item.id === selected) ||
    sessions.find((item) => item.id === snapshot?.selected) ||
    sessions[0];
  const native = snapshot?.sessions.find(
    (item) => item.session_id === session?.id,
  );
  const alive =
    !!native && !["DEAD", "COMPLETED", "EXITED"].includes(native.state);
  const dirty = !!editor && editor.content !== editor.original;
  const showError = (failure: unknown) =>
    setError(failure instanceof Error ? failure.message : String(failure));
  const act = async (action: () => Promise<void>) => {
    setError("");
    try {
      await action();
    } catch (failure) {
      showError(failure);
    }
  };
  useEffect(() => {
    localStorage.setItem(
      "agmux-appearance",
      JSON.stringify({ light, density }),
    );
  }, [light, density]);
  useEffect(() => {
    let cancelled = false;
    backend
      .request<MachineInfo>("/api/info")
      .then((value) => {
        if (!cancelled) {
          setInfo(value);
          setCwd((previous) => previous || value.cwd);
        }
      })
      .catch(showError);
    return () => {
      cancelled = true;
    };
  }, [backend, retry]);
  useEffect(() => {
    let cancelled = false;
    const apply = (value: Snapshot) => {
      if (cancelled) return;
      setSnapshot(value);
      setConnected(true);
      if (!selected && value.selected) setSelected(value.selected);
      if (
        selected &&
        !value.sessions.some((item) => item.session_id === selected)
      )
        setSelected(undefined);
    };
    const query = selected ? `?session_id=${encodeURIComponent(selected)}` : "";
    backend
      .request<Snapshot>("/api/snapshot" + query)
      .then(apply)
      .catch((failure) => {
        if (!cancelled) {
          setConnected(false);
          showError(failure);
          if (selected) setSelected(undefined);
        }
      });
    const events = backend.events(selected);
    const receive = (event: Event) => {
      try {
        apply(JSON.parse((event as MessageEvent).data));
      } catch {
        setError("Invalid update from companion bridge");
      }
    };
    events.addEventListener("snapshot", receive);
    events.addEventListener("update", receive);
    events.addEventListener("offline", () => {
      if (!cancelled) setConnected(false);
    });
    events.onopen = () => {
      if (!cancelled) setConnected(true);
    };
    events.onerror = () => {
      if (!cancelled) setConnected(false);
    };
    return () => {
      cancelled = true;
      events.close();
    };
  }, [backend, selected, retry]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(""), 3200);
    return () => clearTimeout(timer);
  }, [notice]);
  useEffect(() => {
    if (follow.current && output.current)
      output.current.scrollTop = output.current.scrollHeight;
  }, [snapshot?.screen, tab]);
  useEffect(() => {
    if (!modal) return;
    const dialog = document.querySelector<HTMLElement>("[role=dialog]");
    const controls = () =>
      Array.from(
        dialog?.querySelectorAll<HTMLElement>(
          'button:not(:disabled),input,select,textarea,[tabindex="0"]',
        ) || [],
      );
    controls()[0]?.focus();
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") setModal(null);
      if (event.key === "Tab") {
        const items = controls(),
          first = items[0],
          last = items.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [modal]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (modal) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setModal("new");
      }
      if (
        event.altKey &&
        ["ArrowUp", "ArrowDown"].includes(event.key) &&
        sessions.length
      ) {
        event.preventDefault();
        const index = sessions.findIndex((item) => item.id === session?.id);
        void choose(
          sessions[
            (index + (event.key === "ArrowDown" ? 1 : -1) + sessions.length) %
              sessions.length
          ].id,
        );
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [session?.id, sessions, modal, dirty]);
  const refresh = async () => {
    const value = await backend.request<Snapshot>(
      "/api/snapshot" +
        (selected ? "?session_id=" + encodeURIComponent(selected) : ""),
    );
    setSnapshot(value);
  };
  async function choose(id: string) {
    if (busy) {
      setError("Wait for the current operation to finish.");
      return;
    }
    if (dirty) {
      setError("Save or discard your file changes before switching sessions.");
      setTab("Files");
      return;
    }
    const sequence = ++chooseSequence.current;
    await act(async () => {
      await backend.request("/api/select", { session_id: id });
      if (sequence === chooseSequence.current) {
        setSelected(id);
        setEditor(null);
        setDirectory("");
        setError("");
        follow.current = true;
        setReadingBack(false);
        if (window.innerWidth < 850) setSidebar(false);
      }
    });
  }
  const loadFiles = async (path = directory) => {
    if (!session) return;
    const sequence = ++fileSequence.current;
    setFilesLoading(true);
    try {
      const value = await backend.request<{ entries: Entry[] }>("/api/files", {
        session_id: session.id,
        path,
      });
      if (sequence === fileSequence.current) setEntries(value.entries);
    } catch (failure) {
      if (sequence === fileSequence.current) showError(failure);
    } finally {
      if (sequence === fileSequence.current) setFilesLoading(false);
    }
  };
  const loadDiff = async () => {
    if (!session) return;
    setDiffLoading(true);
    try {
      const value = await backend.request<{ files: NativeDiff[] }>(
        "/api/diff",
        { session_id: session.id, scope: diffScope },
      );
      setDiffs(value.files);
    } catch (failure) {
      showError(failure);
      setDiffs([]);
    } finally {
      setDiffLoading(false);
    }
  };
  useEffect(() => {
    if (tab === "Files") void loadFiles();
  }, [session?.id, directory, tab]);
  useEffect(() => {
    if ((tab === "Git" || review) && session) void loadDiff();
  }, [session?.id, diffScope, tab, review]);
  const save = () =>
    act(async () => {
      if (!editor) return;
      setBusy(true);
      try {
        const result = await backend.request<{ version: string }>(
          "/api/file/write",
          {
            panel_id: editor.id,
            content: editor.content,
            version: editor.version,
          },
        );
        setEditor((current) =>
          current?.id === editor.id
            ? { ...current, original: editor.content, version: result.version }
            : current,
        );
        setNotice("File saved to workspace");
        await loadDiff();
      } finally {
        setBusy(false);
      }
    });
  const openFile = (entry: Entry) =>
    act(async () => {
      if (busy) return;
      if (entry.is_dir) {
        setDirectory(entry.path);
        return;
      }
      if (dirty) {
        setError("Save or discard your changes before opening another file.");
        return;
      }
      setBusy(true);
      try {
        const opened = await backend.request<{ panel_id: number }>(
          "/api/panels/open",
          { session_id: session.id, path: entry.path },
        );
        const value = await backend.request<{
          content: string;
          version: string;
        }>("/api/file/read", { panel_id: opened.panel_id });
        setEditor({
          id: opened.panel_id,
          path: entry.path,
          content: value.content,
          original: value.content,
          version: value.version,
        });
      } finally {
        setBusy(false);
      }
    });
  const create = () =>
    act(async () => {
      if (dirty)
        throw new Error(
          "Save or discard file changes before starting another session.",
        );
      setBusy(true);
      try {
        const parsed: unknown = JSON.parse(args);
        if (
          !Array.isArray(parsed) ||
          !parsed.every((value) => typeof value === "string")
        )
          throw new Error("Arguments must be a JSON array of strings.");
        if (newAgent === "gemini" && geminiModel.trim()) {
          if (parsed.some((arg) => arg === "-m" || arg === "--model" || arg.startsWith("--model=")))
            throw new Error("Set the Gemini model in either Model or Arguments, not both.");
          parsed.push("--model", geminiModel.trim());
        }
        const value = await backend.request<{ session_id: string }>(
          "/api/spawn",
          {
            agent: newAgent,
            cwd,
            title: newTitle,
            command: custom,
            args: parsed,
          },
        );
        setSelected(value.session_id);
        setDirectory("");
        setEditor(null);
        setTab("Terminal");
        setModal(null);
        setNewTitle("");
        setNotice("Session started");
      } finally {
        setBusy(false);
      }
    });
  const send = () =>
    act(async () => {
      if (
        !session ||
        !alive ||
        !connected ||
        busy ||
        !drafts[session.id]?.trim()
      )
        return;
      const id = session.id,
        raw = drafts[id],
        message = raw.trim();
      setBusy(true);
      try {
        await backend.request("/api/reply", { session_id: id, message });
        setDrafts((previous) =>
          previous[id] === raw ? { ...previous, [id]: "" } : previous,
        );
        follow.current = true;
        setReadingBack(false);
        await refresh();
      } finally {
        setBusy(false);
      }
    });
  const selectedMetadata = snapshot?.metadata[session?.id] || fallbackMetadata;
  const filtered = sessions.filter((item) =>
    item.title.toLowerCase().includes(search.toLowerCase()),
  );
  const groups = [
    {
      name: "Needs attention",
      items: filtered.filter((item) => item.state === "attention"),
    },
    {
      name: "Working",
      items: filtered.filter((item) => item.state === "working"),
    },
    {
      name: "Pinned",
      items: filtered.filter(
        (item) =>
          item.pinned && item.state !== "working" && item.state !== "attention",
      ),
    },
    {
      name: "Sessions",
      items: filtered.filter(
        (item) =>
          !item.pinned &&
          item.state !== "working" &&
          item.state !== "attention",
      ),
    },
  ];
  const renderDiff = () => (
    <>
      {diffLoading && (
        <p className="live-hint" role="status">
          Loading changes…
        </p>
      )}
      {!diffLoading && !diffs.length && (
        <EmptyState
          compact
          kind="completed"
          title="No tracked changes"
          description="There are no tracked changes for this comparison. Untracked files remain available in Files."
        />
      )}
      {diffs.map((file) => (
        <div className="git-file" key={file.new_path || file.old_path}>
          <div className="diff-file-header">
            <FileCode2 size={14} />
            <span>{file.new_path || file.old_path}</span>
          </div>
          <Diff
            lines={diffLines(file)}
            layout={diffLayout}
            language={
              (file.new_path || file.old_path || "").endsWith(".rs")
                ? "rust"
                : "tsx"
            }
          />
        </div>
      ))}
    </>
  );
  return (
    <MotionConfig reducedMotion="user">
      <div
        className={"app live-app" + (light ? " light" : "")}
        data-theme={light ? "light" : "dark"}
        data-density={density}
      >
        {sidebar && (
          <aside className="sidebar">
            <div className="sidebar-chrome">
              <span className="tiny-badge">LIVE COMPANION</span>
              <button
                className="icon-button"
                aria-label="Hide sidebar"
                onClick={() => setSidebar(false)}
              >
                <PanelLeft size={15} />
              </button>
            </div>
            <div className="brand-row">
              <div className="brand-mark">⌘</div>
              <div className="brand-copy">
                <div>
                  <strong>Agmux</strong>
                </div>
                <span>
                  <i
                    className={"status-dot " + (connected ? "green" : "amber")}
                  />
                  {connected
                    ? "Connected to Rust daemon"
                    : "Reconnecting to daemon"}
                </span>
              </div>
              <button
                className="icon-button"
                aria-label="New session"
                onClick={() => setModal("new")}
              >
                <Plus size={21} />
              </button>
            </div>
            <div className="search-box">
              <Search size={14} />
              <input
                aria-label="Search sessions"
                placeholder="Search sessions…"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <SessionList
              density={density}
              groups={groups}
              selected={session?.id || ""}
              collapsed={collapsed}
              onCollapse={(name) =>
                setCollapsed((previous) => ({
                  ...previous,
                  [name]: !previous[name],
                }))
              }
              onSelect={(id) => void choose(id)}
              onFilter={() => setSearch("")}
              onClear={() => setSearch("")}
              icon={(item) => <AgentIcon agent={item.agent} />}
            />
            <div className="sidebar-footer">
              <button className="new-session" onClick={() => setModal("new")}>
                <Plus size={15} />
                Start new session<kbd>Ctrl N</kbd>
              </button>
              <div className="footer-links">
                <button onClick={() => setTab("Machines")}>
                  <Monitor size={14} />
                  {info?.machine || "Local machine"}
                </button>
                <button
                  className="icon-button"
                  aria-label="Settings"
                  onClick={() => setModal("settings")}
                >
                  <Settings size={16} />
                </button>
              </div>
            </div>
          </aside>
        )}
        <main className="workspace">
          <header className="workspace-header">
            <div className="row header-title">
              {!sidebar && (
                <button
                  className="icon-button"
                  aria-label="Show sidebar"
                  onClick={() => setSidebar(true)}
                >
                  <Menu size={18} />
                </button>
              )}
              <div>
                <h1>{session?.title || "Your workspace"}</h1>
                <span className="workspace-subtitle">
                  <Folder size={12} />
                  {native?.cwd || "Start a real session to begin"}
                </span>
              </div>
            </div>
            <div className="row header-actions">
              {session && (
                <>
                  <button
                    className="icon-button"
                    aria-label={
                      session.pinned ? "Unpin session" : "Pin session"
                    }
                    onClick={() =>
                      void act(async () => {
                        await backend.request("/api/workspace", {
                          session_id: session.id,
                          metadata: {
                            ...selectedMetadata,
                            favorite: !session.pinned,
                          },
                        });
                        await refresh();
                      })
                    }
                  >
                    <Check size={16} />
                  </button>
                  <button
                    className="live-action"
                    disabled={!alive || !connected}
                    onClick={() =>
                      void act(async () => {
                        await backend.request("/api/interrupt", {
                          session_id: session.id,
                        });
                        setNotice("Interrupt sent to this session");
                      })
                    }
                  >
                    Interrupt
                  </button>
                  <button
                    className="live-action"
                    disabled={!alive || !connected}
                    onClick={() =>
                      void act(async () => {
                        await backend.request("/api/freeze", {
                          session_id: session.id,
                        });
                        setNotice("Session ended · snapshot retained");
                        await refresh();
                      })
                    }
                  >
                    End session
                  </button>
                </>
              )}
              <button
                className="icon-button"
                aria-label="Toggle review panel"
                onClick={() => setReview(!review)}
              >
                <PanelRight size={17} />
              </button>
              <button
                className="icon-button"
                aria-label="Settings"
                onClick={() => setModal("settings")}
              >
                <Settings size={16} />
              </button>
            </div>
          </header>
          {!connected && snapshot && (
            <div className="connection-banner" role="status">
              Connection interrupted. Your draft is safe; input is disabled
              until the daemon reconnects.
              <button onClick={() => setRetry((value) => value + 1)}>
                Reconnect
              </button>
            </div>
          )}
          {error && (
            <div className="live-error" role="alert">
              <span>{error}</span>
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                <X size={14} />
              </button>
            </div>
          )}
          <div className="workspace-body">
            <section className="conversation-pane">
              <nav className="view-tabs" aria-label="Workspace views">
                {(
                  [
                    { name: "Chat", icon: MessageSquare },
                    { name: "Terminal", icon: SquareTerminal },
                    { name: "Files", icon: FileCode2 },
                    { name: "Git", icon: GitBranch },
                    { name: "Machines", icon: Monitor },
                  ] as const
                ).map((item) => (
                  <button
                    key={item.name}
                    disabled={!session && item.name !== "Machines"}
                    className={tab === item.name ? "active" : ""}
                    onClick={() => setTab(item.name)}
                  >
                    {tab === item.name && (
                      <motion.span
                        layoutId="live-tab"
                        className="active-tab-indicator"
                        transition={motionTiming(reduced, "navigation")}
                      />
                    )}
                    <item.icon size={14} />
                    {item.name}
                  </button>
                ))}
              </nav>
              {!snapshot ? (
                <div className="live-loading">
                  <span className="working-dots">
                    <i />
                    <i />
                    <i />
                  </span>
                  <p>Connecting to your workspace…</p>
                  <button
                    className="live-action"
                    onClick={() => setRetry((value) => value + 1)}
                  >
                    Retry connection
                  </button>
                </div>
              ) : !session && tab !== "Machines" ? (
                <EmptyState
                  kind="workspace"
                  title="Your next idea starts here"
                  description="Launch a real shell or an installed coding agent. Sessions run in the Rust daemon and survive closing this tab."
                  action="Start your first session"
                  onAction={() => setModal("new")}
                />
              ) : (
                <>
                  {tab === "Chat" && (
                    <>
                      <div className="live-chat-header">
                        <AgentIcon agent={session.agent} />
                        <strong>{session.agent}</strong>
                        <span>{native?.state}</span>
                        <button
                          className="live-action"
                          onClick={() => {
                            setRecipient(
                              sessions.find((item) => item.id !== session.id)
                                ?.id || "",
                            );
                            setModal("send");
                          }}
                        >
                          Send to session
                        </button>
                      </div>
                      <div className="live-output-shell">
                        <pre
                          ref={output}
                          className="live-output"
                          aria-label="Live session output"
                          onScroll={() => {
                            const element = output.current!;
                            follow.current =
                              element.scrollHeight -
                                element.scrollTop -
                                element.clientHeight <
                              60;
                            setReadingBack(!follow.current);
                          }}
                        >
                          {snapshot.screen ||
                            "Waiting for output from this process…"}
                        </pre>
                        {readingBack && (
                          <button
                            className="jump-latest"
                            onClick={() => {
                              follow.current = true;
                              setReadingBack(false);
                              if (output.current)
                                output.current.scrollTop =
                                  output.current.scrollHeight;
                            }}
                          >
                            Latest output
                          </button>
                        )}
                      </div>
                      <p className="live-hint">
                        Live terminal transcript · approval prompts are answered
                        in Terminal. Structured tool messages depend on the
                        agent adapter.
                      </p>
                      <Composer
                        session={{
                          ...session,
                          state: connected ? session.state : "reconnecting",
                        }}
                        sessions={sessions}
                        draft={drafts[session.id] || ""}
                        onDraft={(text) =>
                          setDrafts((previous) => ({
                            ...previous,
                            [session.id]: text,
                          }))
                        }
                        onSend={() => void send()}
                        running={false}
                        onStop={() => {}}
                        mode="CLI settings"
                        onMode={() =>
                          setNotice(
                            "Modes and permissions are controlled by this agent in Terminal",
                          )
                        }
                        onAgent={() => setModal("new")}
                        agentIcon={<AgentIcon agent={session.agent} />}
                        queue={[]}
                        onQueueMove={() => {}}
                        onQueueDelete={() => {}}
                        onQueueEdit={() => {}}
                        onQueueResume={() => {}}
                        attachments={[]}
                        onAttach={() =>
                          setNotice(
                            "Use workspace file paths in prompts; uploads are not supported by this agent bridge",
                          )
                        }
                        onRemoveAttachment={() => {}}
                        onLatest={() => {
                          follow.current = true;
                          if (output.current)
                            output.current.scrollTop =
                              output.current.scrollHeight;
                        }}
                        onSettings={() => setModal("settings")}
                        live
                        disabled={!alive || busy}
                      />
                    </>
                  )}
                  {tab === "Terminal" && (
                    <Suspense
                      fallback={<p className="live-hint">Opening terminal…</p>}
                    >
                      {alive ? (
                        <LiveTerminal
                          backend={backend}
                          sessionId={session.id}
                          light={light}
                        />
                      ) : (
                        <div className="live-output-shell">
                          <div className="live-hint">
                            Archived output · this process has ended
                          </div>
                          <pre className="live-output">
                            {snapshot.screen || "No saved screen is available."}
                          </pre>
                        </div>
                      )}
                    </Suspense>
                  )}
                  {tab === "Files" && (
                    <div className="files-view">
                      <div className="file-tree">
                        <div className="live-file-heading">
                          <span>WORKSPACE</span>
                          <button
                            className="icon-button"
                            aria-label="Refresh files"
                            onClick={() => void loadFiles()}
                          >
                            <Zap size={13} />
                          </button>
                        </div>
                        <button
                          disabled={!directory}
                          onClick={() =>
                            setDirectory(
                              directory
                                .replaceAll("\\", "/")
                                .split("/")
                                .slice(0, -1)
                                .join("/"),
                            )
                          }
                        >
                          <ArrowLeft size={12} />
                          Parent directory
                        </button>
                        <p className="live-directory">{directory || "/"}</p>
                        {filesLoading ? (
                          <p className="live-hint">Loading…</p>
                        ) : (
                          entries.map((entry) => (
                            <button
                              key={entry.path}
                              className={
                                editor?.path === entry.path ? "active" : ""
                              }
                              onClick={() => void openFile(entry)}
                            >
                              {entry.is_dir ? (
                                <Folder size={13} />
                              ) : (
                                <FileCode2 size={13} />
                              )}
                              <span>{entry.name}</span>
                            </button>
                          ))
                        )}
                      </div>
                      <div className="editor-pane">
                        {editor ? (
                          <>
                            <div className="editor-tab">
                              <FileCode2 size={14} />
                              <span>
                                {editor.path}
                                {dirty ? " •" : ""}
                              </span>
                              <button
                                disabled={busy || !dirty}
                                onClick={() => void save()}
                              >
                                Save
                              </button>
                              <button
                                disabled={!dirty}
                                onClick={() =>
                                  setEditor({
                                    ...editor,
                                    content: editor.original,
                                  })
                                }
                              >
                                Discard
                              </button>
                            </div>
                            <Suspense
                              fallback={
                                <p className="live-hint">Opening editor…</p>
                              }
                            >
                              <CodeEditor
                                value={editor.content}
                                file={editor.path}
                                light={light}
                                onChange={(content) =>
                                  setEditor((previous) =>
                                    previous
                                      ? { ...previous, content }
                                      : previous,
                                  )
                                }
                                onSave={() => void save()}
                              />
                            </Suspense>
                          </>
                        ) : (
                          <EmptyState
                            kind="workspace"
                            title="A closer look at your code"
                            description="Choose a file from this session's workspace. Saved edits are written to disk through the Rust daemon."
                          />
                        )}
                      </div>
                    </div>
                  )}
                  {tab === "Git" && (
                    <div className="git-view">
                      <div className="git-intro">
                        <GitBranch size={22} />
                        <div>
                          <h2>Workspace changes</h2>
                          <p>{native?.cwd}</p>
                        </div>
                      </div>
                      <div className="git-view-controls">
                        <select
                          aria-label="Diff scope"
                          value={diffScope}
                          onChange={(event) => setDiffScope(event.target.value)}
                        >
                          <option value="FullUncommitted">Uncommitted</option>
                          <option value="Staged">Staged</option>
                          <option value="LastCommit">Last commit</option>
                        </select>
                        <div className="segmented-control">
                          {(["unified", "split"] as const).map((layout) => (
                            <button
                              key={layout}
                              aria-pressed={layout === diffLayout}
                              onClick={() => setDiffLayout(layout)}
                            >
                              <Selection
                                active={layout === diffLayout}
                                id="live-diff-layout"
                              />
                              {layout === "unified"
                                ? "Unified"
                                : "Side by side"}
                            </button>
                          ))}
                        </div>
                        <button
                          className="live-action"
                          onClick={() => void loadDiff()}
                        >
                          Refresh
                        </button>
                      </div>
                      {renderDiff()}
                    </div>
                  )}
                  {tab === "Machines" && (
                    <div className="machines-view">
                      <div className="page-intro">
                        <span className="eyebrow">Your code runs here</span>
                        <h2>{info?.machine || "Local machine"}</h2>
                        <p>Rust daemon · Agmux {info?.version}</p>
                      </div>
                      <div className="machine-preview-card">
                        <Monitor size={28} />
                        <div>
                          <h3>{connected ? "Connected" : "Disconnected"}</h3>
                          <p>{info?.cwd}</p>
                          <span className="machine-status">
                            Authenticated local companion
                          </span>
                        </div>
                        <button onClick={() => setRetry((value) => value + 1)}>
                          Refresh connection
                        </button>
                      </div>
                      <div className="live-agent-list">
                        <h3>Installed agent CLIs</h3>
                        {info?.agents.map((agent) => (
                          <div key={agent.id}>
                            <AgentIcon agent={agent.name} />
                            <strong>{agent.name}</strong>
                            <code>{agent.command}</code>
                            <span>
                              {agent.available ? "Available" : "Not installed"}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </section>
            {review && session && (
              <>
                <PanelDivider width={reviewWidth} onChange={setReviewWidth} />
                <aside className="review-pane" style={{ width: reviewWidth }}>
                  <div className="review-top">
                    <span className="review-tab">
                      <GitBranch size={13} />
                      Actual workspace changes
                    </span>
                    <button
                      className="icon-button"
                      aria-label="Close review"
                      onClick={() => setReview(false)}
                    >
                      <X size={15} />
                    </button>
                  </div>
                  <div className="review-toolbar">
                    <span>
                      {diffs.length} file{diffs.length === 1 ? "" : "s"}
                    </span>
                    <button
                      className="live-action"
                      onClick={() => void loadDiff()}
                    >
                      Refresh
                    </button>
                  </div>
                  <div className="review-scroll">{renderDiff()}</div>
                </aside>
              </>
            )}
          </div>
          <footer className="status-bar">
            <span>
              <i className={"status-dot " + (connected ? "green" : "amber")} />
              {connected ? "Live daemon" : "Disconnected"}
            </span>
            <span>{sessions.length} sessions</span>
            <span className="status-spacer" />
            <span>Local process execution · no simulation</span>
          </footer>
        </main>
        <AnimatePresence>
          {notice && (
            <motion.div
              className="toast"
              role="status"
              style={{ x: "-50%" }}
              initial={{ opacity: 0, y: reduced ? 0 : 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
            >
              <AnimatedCheck />
              {notice}
              <button
                aria-label="Dismiss notification"
                onClick={() => setNotice("")}
              >
                <X size={13} />
              </button>
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence>
          {modal && (
            <AnimatedOverlay
              onMouseDown={(event) => {
                if (event.target === event.currentTarget) setModal(null);
              }}
            >
              <motion.section
                className="modal"
                role="dialog"
                aria-modal="true"
                aria-label={
                  modal === "new"
                    ? "New session"
                    : modal === "send"
                      ? "Send to session"
                      : "Settings"
                }
                initial={{ opacity: 0, scale: reduced ? 1 : 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0 }}
              >
                <div className="modal-heading">
                  <h2>
                    {modal === "new"
                      ? "Start a real session"
                      : modal === "send"
                        ? "Send to another session"
                        : "Workspace settings"}
                  </h2>
                  <button
                    className="icon-button"
                    aria-label="Close dialog"
                    onClick={() => setModal(null)}
                  >
                    <X size={17} />
                  </button>
                </div>
                {error && <p className="live-error">{error}</p>}
                {modal === "new" && (
                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      void create();
                    }}
                  >
                    <p className="modal-description">
                      Runs the selected CLI on this machine using its existing
                      login and configuration.
                    </p>
                    <label>
                      Session name
                      <input
                        aria-label="Session name"
                        value={newTitle}
                        onChange={(event) => setNewTitle(event.target.value)}
                        placeholder="What are you working on?"
                      />
                    </label>
                    <label>
                      Project directory
                      <input
                        required
                        aria-label="Project directory"
                        value={cwd}
                        onChange={(event) => setCwd(event.target.value)}
                      />
                    </label>
                    <label>
                      Agent
                      <select
                        aria-label="Agent"
                        value={newAgent}
                        onChange={(event) => setNewAgent(event.target.value)}
                      >
                        {info?.agents.map((agent) => (
                          <option
                            key={agent.id}
                            value={agent.id}
                            disabled={!agent.available && agent.id !== "gemini"}
                          >
                            {agent.name}
                            {agent.available ? "" : " · not installed"}
                          </option>
                        ))}
                        <option value="custom">Custom executable</option>
                      </select>
                    </label>
                    {newAgent === "gemini" && (
                      <div className="gemini-provider-setup">
                        <p className="live-hint">Gemini CLI runs locally. Sign in and answer permission prompts in Terminal.</p>
                        {!info?.agents.find((agent) => agent.id === "gemini")?.available && (
                          <p role="status">Install Node.js 20+ and run <code>npm install -g @google/gemini-cli</code>, then restart Agmux if PATH changed.</p>
                        )}
                        <button type="button" className="live-action" onClick={() => void act(async () => setInfo(await backend.request<MachineInfo>("/api/info")))}>Refresh installed agents</button>
                        <label>Model (optional)
                          <input aria-label="Gemini model" value={geminiModel} onChange={(event) => setGeminiModel(event.target.value)} placeholder="Use Gemini CLI's configured default" />
                        </label>
                        <p className="live-hint">Leave Model blank to use your CLI settings. Use Arguments for options such as --resume. No API key is stored in this UI.</p>
                      </div>
                    )}
                    {newAgent === "custom" && (
                      <label>
                        Executable
                        <input
                          required
                          value={custom}
                          onChange={(event) => setCustom(event.target.value)}
                          placeholder="/path/to/agent"
                        />
                      </label>
                    )}
                    <label>
                      Arguments (JSON array)
                      <input
                        value={args}
                        onChange={(event) => setArgs(event.target.value)}
                        placeholder={'["--flag"]'}
                      />
                    </label>
                    <div className="modal-footer">
                      <button
                        type="button"
                        className="live-action"
                        onClick={() => setModal(null)}
                      >
                        Cancel
                      </button>
                      <button
                        className="primary-button"
                        disabled={busy || !connected || !info || (newAgent !== "custom" && !info?.agents.find((agent) => agent.id === newAgent)?.available)}
                      >
                        {busy ? "Starting…" : "Start session"}
                        <ArrowRight size={14} />
                      </button>
                    </div>
                  </form>
                )}
                {modal === "send" && (
                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      void act(async () => {
                        if (!recipient || !outgoing.trim()) return;
                        const result = await backend.request<{
                          queued: boolean;
                        }>("/api/send", {
                          session_id: recipient,
                          message: outgoing,
                        });
                        setNotice(
                          result.queued
                            ? "Queued by daemon until the receiving session is acknowledged"
                            : "Message delivered to receiving session",
                        );
                        setOutgoing("");
                        setModal(null);
                      });
                    }}
                  >
                    <p className="modal-description">
                      The daemon controls delivery and queueing. This does not
                      approve a tool permission.
                    </p>
                    <label>
                      Recipient
                      <select
                        value={recipient}
                        required
                        onChange={(event) => setRecipient(event.target.value)}
                      >
                        <option value="">Choose session</option>
                        {sessions
                          .filter((item) => item.id !== session?.id)
                          .map((item) => (
                            <option value={item.id} key={item.id}>
                              {item.title}
                            </option>
                          ))}
                      </select>
                    </label>
                    <label>
                      Message
                      <textarea
                        required
                        value={outgoing}
                        onChange={(event) => setOutgoing(event.target.value)}
                        rows={4}
                      />
                    </label>
                    <div className="modal-footer">
                      <button
                        className="primary-button"
                        disabled={!recipient || !connected}
                      >
                        Send message
                      </button>
                    </div>
                  </form>
                )}
                {modal === "settings" && (
                  <>
                    <p className="modal-description">
                      This UI is connected to your actual Rust daemon. Agent
                      login, model, mode, and tool permissions are managed by
                      each CLI in Terminal.
                    </p>
                    <div className="setting-row">
                      <div>
                        <strong>Appearance</strong>
                        <span>Local preference</span>
                      </div>
                      <div className="segmented-control">
                        <button
                          aria-pressed={!light}
                          onClick={() => setLight(false)}
                        >
                          Dark
                        </button>
                        <button
                          aria-pressed={light}
                          onClick={() => setLight(true)}
                        >
                          Light
                        </button>
                      </div>
                    </div>
                    <div className="setting-row">
                      <div>
                        <strong>Density</strong>
                      </div>
                      <div className="segmented-control">
                        {(["comfortable", "compact"] as const).map((value) => (
                          <button
                            key={value}
                            aria-pressed={value === density}
                            onClick={() => setDensity(value)}
                          >
                            {value}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div className="modal-footer">
                      <button
                        className="primary-button"
                        onClick={() => setModal(null)}
                      >
                        Done
                      </button>
                    </div>
                  </>
                )}
              </motion.section>
            </AnimatedOverlay>
          )}
        </AnimatePresence>
      </div>
    </MotionConfig>
  );
}
