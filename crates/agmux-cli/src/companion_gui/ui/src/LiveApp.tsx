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
import CommandPalette, { type WorkspaceCommand } from "./CommandPalette";
import GeminiChat, { type ChatState, type Permission } from "./GeminiChat";
import { diffLines as compareLines } from "diff";
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
        "gemini.exe": "Gemini",
        "gemini.cmd": "Gemini",
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
        : ["DEAD", "COMPLETED", "EXITED", "ARCHIVED"].includes(session.state)
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
      saved("agmux-appearance", { density: "compact" }).density as
        | "comfortable"
        | "compact",
    );
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({}),
    [drafts, setDrafts] = useState<Record<string, string>>({});
  const [modal, setModal] = useState<"new" | "settings" | "send" | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const [paletteMode, setPaletteMode] = useState<
    "commands" | "sessions" | "shortcuts" | null
  >(null);
  const chord = useRef(0);
  const paletteOrigin = useRef<HTMLElement | null>(null);
  const openPalette = (mode: "commands" | "sessions" | "shortcuts") => {
    if (!paletteMode)
      paletteOrigin.current =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
    setPaletteMode(mode);
  };
  const closePalette = () => {
    setPaletteMode(null);
    // An exiting animated dialog can still own the modal layer for a frame.
    // Restore after it leaves, without stealing focus from a newly opened dialog.
    const deadline = performance.now() + 1000;
    const restore = () => {
      if (document.querySelector('[role="dialog"]')) {
        if (performance.now() < deadline) requestAnimationFrame(restore);
        return;
      }
      if (paletteOrigin.current?.isConnected) paletteOrigin.current.focus();
    };
    requestAnimationFrame(restore);
  };
  const [newAgent, setNewAgent] = useState(""),
    [newTitle, setNewTitle] = useState(""),
    [cwd, setCwd] = useState(""),
    [custom, setCustom] = useState(""),
    [args, setArgs] = useState("[]"),
    [geminiModel, setGeminiModel] = useState("");
  const [models, setModels] = useState<{
    availableModels: { modelId: string; name: string }[];
    currentModelId: string;
  } | null>(null);
  const [modelsError, setModelsError] = useState(""),
    [modelsLoading, setModelsLoading] = useState(false),
    [modelRetry, setModelRetry] = useState(0);
  useEffect(() => {
    let cancelled = false;
    setModels(null);
    setModelsError("");
    setGeminiModel("");
    setModelsLoading(false);
    if (
      modal !== "new" ||
      newAgent !== "gemini" ||
      !cwd ||
      !info?.agents.find((a) => a.id === "gemini")?.available
    )
      return;
    const timer = window.setTimeout(async () => {
      setModelsLoading(true);
      try {
        const value = await backend.request<{
          availableModels: { modelId: string; name: string }[];
          currentModelId: string;
        } | null>("/api/models", { cwd });
        if (!cancelled) {
          setModels(value);
          if (!value?.availableModels?.length)
            setModelsError(
              "This CLI did not advertise a model inventory. Its configured default is available.",
            );
        }
      } catch (e) {
        if (!cancelled)
          setModelsError(
            `Model discovery unavailable: ${e instanceof Error ? e.message : String(e)}. Sign in through the CLI and retry, or use its configured default.`,
          );
      } finally {
        if (!cancelled) setModelsLoading(false);
      }
    }, 500);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [modal, newAgent, cwd, info, backend, modelRetry]);
  const agents = info?.agents.filter((agent) => agent.id !== "shell") || [];
  useEffect(() => {
    if (newAgent === "custom") return;
    if (!agents.some((a) => a.id === newAgent))
      setNewAgent(agents.find((a) => a.available)?.id || "");
  }, [info, newAgent]);
  const [directory, setDirectory] = useState(""),
    [entries, setEntries] = useState<Entry[]>([]),
    [editor, setEditor] = useState<OpenFile | null>(null),
    [filesLoading, setFilesLoading] = useState(false);
  const [diffs, setDiffs] = useState<NativeDiff[]>([]),
    [diffError, setDiffError] = useState(""),
    [isRepository, setIsRepository] = useState(true),
    [diffScope, setDiffScope] = useState("FullUncommitted"),
    [diffLayout, setDiffLayout] = useState<"unified" | "split">("split"),
    [diffLoading, setDiffLoading] = useState(false);
  const [recipient, setRecipient] = useState(""),
    [outgoing, setOutgoing] = useState("");
  const chooseSequence = useRef(0),
    output = useRef<HTMLPreElement>(null),
    follow = useRef(true),
    fileSequence = useRef(0),
    diffSequence = useRef(0);
  const [readingBack, setReadingBack] = useState(false);
  const [reviewChats, setReviewChats] = useState<Record<string, ChatState>>({});
  const [reviewDecision, setReviewDecision] = useState(false);
  const [chatStatuses, setChatStatuses] = useState<Record<string, string>>({});
  useEffect(() => {
    if (snapshot?.chat_states) setChatStatuses(snapshot.chat_states);
  }, [snapshot?.chat_states]);
  const sessions = useMemo(
    () =>
      (snapshot?.sessions || []).map((item) => {
        const session = normalizeSession(
          item,
          snapshot?.metadata[item.session_id],
        );
        if (session.agent === "Gemini" && session.state !== "stopped") {
          const status = chatStatuses[session.id] || "idle";
          session.state =
            status === "working" || status === "cancelling"
              ? "working"
              : status === "permission" || status === "failed"
                ? "attention"
                : "idle";
          session.time = status;
        }
        return session;
      }),
    [snapshot?.sessions, snapshot?.metadata, chatStatuses],
  );
  const session =
    sessions.find((item) => item.id === selected) ||
    sessions.find((item) => item.id === snapshot?.selected) ||
    sessions[0];
  const native = snapshot?.sessions.find(
    (item) => item.session_id === session?.id,
  );
  const alive =
    !!native &&
    !["DEAD", "COMPLETED", "EXITED", "ARCHIVED"].includes(native.state);
  const [sessionView, setSessionView] = useState<"activity" | "projects">(() =>
    saved("agmux-session-view", "activity"),
  );
  const [showArchived, setShowArchived] = useState(false);
  useEffect(
    () =>
      localStorage.setItem("agmux-session-view", JSON.stringify(sessionView)),
    [sessionView],
  );
  const managed =
    !!session && !!snapshot?.managed_sessions?.includes(session.id);
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
      if (modal || event.isComposing) return;
      const mod = event.ctrlKey || event.metaKey,
        key = event.key.toLowerCase();
      // Re-pressing Ctrl/Cmd between chord keys must not cancel the chord.
      if (["control", "meta", "shift", "alt"].includes(key)) return;
      if (mod && event.shiftKey && key === "p") {
        event.preventDefault();
        openPalette("commands");
        return;
      }
      if (paletteMode) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          closePalette();
        }
        return;
      }
      const target = event.target instanceof HTMLElement ? event.target : null;
      const nativeTerminal = !!target?.closest(".xterm");
      if (nativeTerminal) return;
      if (mod && key === "p") {
        event.preventDefault();
        openPalette("sessions");
        return;
      }
      if (mod && key === "b") {
        event.preventDefault();
        setSidebar((v) => !v);
        return;
      }
      if (mod && event.code === "Backquote") {
        event.preventDefault();
        setTab((t) => (t === "Terminal" ? "Chat" : "Terminal"));
        return;
      }
      if (mod && key === "s" && Date.now() - chord.current < 1500) {
        event.preventDefault();
        chord.current = 0;
        openPalette("shortcuts");
        return;
      }
      if (mod && key === "k") {
        event.preventDefault();
        chord.current = Date.now();
        return;
      }
      chord.current = 0;
      if (target?.closest('input,textarea,[contenteditable="true"]')) return;
      if (event.key === "F1") {
        event.preventDefault();
        openPalette("commands");
        return;
      }
      if (mod && key === "n") {
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
  }, [session?.id, sessions, modal, paletteMode, dirty]);
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
  const diffBusy = useRef(false);
  const loadDiff = async (background = false) => {
    if (!session) return;
    if (background && diffBusy.current) return;
    diffBusy.current = true;
    const sequence = ++diffSequence.current;
    if (!background) setDiffLoading(true);
    try {
      const value = await backend.request<{
        files: NativeDiff[];
        repository: boolean;
      }>("/api/diff", { session_id: session.id, scope: diffScope });
      if (sequence !== diffSequence.current) return;
      setDiffs((previous) =>
        JSON.stringify(previous) === JSON.stringify(value.files)
          ? previous
          : value.files,
      );
      setIsRepository(value.repository);
      setDiffError("");
    } catch (failure) {
      if (sequence !== diffSequence.current) return;
      setDiffError(
        failure instanceof Error ? failure.message : String(failure),
      );
      setDiffs([]);
    } finally {
      if (sequence === diffSequence.current) {
        setDiffLoading(false);
        diffBusy.current = false;
      }
    }
  };
  useEffect(() => {
    if (tab === "Files") void loadFiles();
  }, [session?.id, directory, tab]);
  useEffect(() => {
    if ((tab === "Git" || review) && session) void loadDiff();
    if (!session || (tab !== "Git" && !review)) return;
    const timer = window.setInterval(() => void loadDiff(true), 1000);
    return () => clearInterval(timer);
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
          if (
            parsed.some(
              (arg) =>
                arg === "-m" || arg === "--model" || arg.startsWith("--model="),
            )
          )
            throw new Error(
              "Set the Gemini model in either Model or Arguments, not both.",
            );
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
        setTab(newAgent === "gemini" ? "Chat" : "Terminal");
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
  const projectPath = (id: string) => {
    let value = snapshot?.sessions.find((s) => s.session_id === id)?.cwd || "";
    if (value.startsWith("\\\\?\\UNC\\")) value = "\\\\" + value.slice(8);
    else if (value.startsWith("\\\\?\\")) value = value.slice(4);
    return value
      .replaceAll("\\", "/")
      .replace(/^[a-z]:/, (s) => s.toUpperCase())
      .replace(/\/$/, "");
  };
  const isArchived = (id: string) => !!snapshot?.metadata[id]?.archived;
  const activity = (id: string) =>
    snapshot?.chat_activity?.[id] ||
    (snapshot?.sessions.find((s) => s.session_id === id)?.created_at || 0) *
      1000;
  const filtered = sessions
    .filter((s) =>
      (s.title + " " + projectPath(s.id))
        .toLowerCase()
        .includes(search.toLowerCase()),
    )
    .sort(
      (a, b) => activity(b.id) - activity(a.id) || a.id.localeCompare(b.id),
    );
  const active = filtered.filter(
    (s) => s.state !== "stopped" && !isArchived(s.id),
  );
  const inactive = filtered.filter(
    (s) => s.state === "stopped" && !isArchived(s.id),
  );
  const archived = filtered.filter((s) => isArchived(s.id));
  const projects = new Map<string, Session[]>();
  for (const item of active) {
    const path = projectPath(item.id);
    projects.set(path, [...(projects.get(path) || []), item]);
  }
  const projectLabel = (full: string) => {
    const parts = full.split("/");
    for (let count = 2; count <= parts.length; count++) {
      const suffix = parts.slice(-count).join("/");
      if (
        ![...projects.keys()].some(
          (other) =>
            other !== full &&
            (other === suffix || other.endsWith("/" + suffix)),
        )
      )
        return suffix === full ? full : "…/" + suffix;
    }
    return full;
  };
  const groups = [
    ...(sessionView === "activity"
      ? [{ name: "Active", items: active }]
      : [...projects]
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([name, items]) => ({
            name,
            label: projectLabel(name),
            items,
          }))),
    { name: "Inactive", items: inactive },
    ...(showArchived ? [{ name: "Archived", items: archived }] : []),
  ];
  const archiveSession = () =>
    void act(async () => {
      if (!session || alive) return;
      const archived = !isArchived(session.id);
      await backend.request("/api/workspace", {
        session_id: session.id,
        metadata: {
          ...(snapshot?.metadata[session.id] || fallbackMetadata),
          archived,
        },
      });
      setShowArchived(archived);
      setNotice(
        archived
          ? "Session archived · history retained"
          : "Session restored to Inactive",
      );
      await refresh();
    });
  const workspaceCommands: WorkspaceCommand[] = [
    {
      id: "new",
      label: "New agent session",
      shortcut: "Ctrl/⌘ N",
      run: () => setModal("new"),
    },
    { id: "chat", label: "Show Chat", run: () => setTab("Chat") },
    {
      id: "terminal",
      label: "Show Terminal",
      shortcut: "Ctrl/⌘ `",
      run: () => setTab("Terminal"),
    },
    { id: "git", label: "Show Git changes", run: () => setTab("Git") },
    {
      id: "sidebar",
      label: "Toggle sidebar",
      shortcut: "Ctrl/⌘ B",
      run: () => setSidebar((v) => !v),
    },
    {
      id: "review",
      label: "Toggle review panel",
      run: () => setReview((v) => !v),
    },
    {
      id: "activity",
      label: "Activity session view",
      run: () => setSessionView("activity"),
    },
    {
      id: "projects",
      label: "Project session view",
      run: () => setSessionView("projects"),
    },
    {
      id: "compact",
      label: "Toggle compact session rows",
      run: () =>
        setDensity((v) => (v === "compact" ? "comfortable" : "compact")),
    },
    {
      id: "quick",
      label: "Quick session switcher",
      shortcut: "Ctrl/⌘ P",
      run: () => openPalette("sessions"),
    },
    {
      id: "shortcuts",
      label: "Keyboard shortcuts",
      shortcut: "Ctrl/⌘ K, Ctrl/⌘ S",
      run: () => openPalette("shortcuts"),
    },
    {
      id: "settings",
      label: "Workspace settings",
      run: () => setModal("settings"),
    },
  ];
  const paletteCommands =
    paletteMode === "sessions"
      ? sessions
          .filter((s) => showArchived || !isArchived(s.id))
          .sort((a, b) => activity(b.id) - activity(a.id))
          .map((s) => ({
            id: s.id,
            label: s.title,
            detail: `${projectPath(s.id)} · ${s.state === "stopped" ? "Inactive" : s.time}`,
            run: () => void choose(s.id),
          }))
      : workspaceCommands;
  const pendingPermissions = managed
    ? reviewChats[session.id]?.permissions || []
    : [];
  const decide = (permission: Permission, optionId: string) =>
    void act(async () => {
      setReviewDecision(true);
      try {
        const state = await backend.request<ChatState>("/api/chat", {
          session_id: session.id,
          operation: "permission",
          id: permission.id,
          option_id: optionId,
        });
        setReviewChats((p) => ({ ...p, [session.id]: state }));
      } finally {
        setReviewDecision(false);
      }
    });
  const proposalLines = (
    oldText: string | null,
    newText: string,
  ): DiffLine[] => {
    let old = 0,
      next = 0;
    const lines: DiffLine[] = [];
    for (const part of compareLines(oldText || "", newText))
      for (const text of part.value
        .split("\n")
        .slice(0, part.value.endsWith("\n") ? -1 : undefined)) {
        if (part.added) lines.push({ kind: "add", text, next: ++next });
        else if (part.removed) lines.push({ kind: "del", text, old: ++old });
        else lines.push({ kind: "context", text, old: ++old, next: ++next });
      }
    return lines;
  };
  const renderDiff = () => (
    <>
      {pendingPermissions.length > 0 && (
        <section className="proposed-changes" aria-label="Proposed changes">
          <h3>Proposed · awaiting approval</h3>
          {pendingPermissions.map((permission, index) => (
            <div key={index} className="proposed-edit">
              <strong>{permission.title}</strong>
              {Array.isArray(permission.tool_call?.content) &&
                permission.tool_call.content
                  .filter(
                    (block) =>
                      block.type === "diff" &&
                      typeof block.path === "string" &&
                      typeof block.newText === "string",
                  )
                  .map((block, i) => (
                    <div key={i}>
                      <div className="diff-file-header">{block.path}</div>
                      <Diff
                        lines={proposalLines(block.oldText, block.newText)}
                        layout="split"
                        language={block.path.endsWith(".rs") ? "rust" : "tsx"}
                      />
                    </div>
                  ))}
              {!permission.tool_call?.content?.some(
                (b) => b.type === "diff",
              ) && (
                <p className="live-hint">
                  The provider has not supplied a text diff for this operation.
                </p>
              )}
              <div className="permission-actions">
                {permission.options.map((option) => (
                  <button
                    key={option.optionId}
                    disabled={reviewDecision}
                    className="live-action"
                    onClick={() => decide(permission, option.optionId)}
                  >
                    {option.name}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </section>
      )}
      <h3 className="applied-heading">Applied · on disk</h3>
      {diffLoading && (
        <p className="live-hint" role="status">
          Loading changes…
        </p>
      )}
      {diffError && (
        <details className="diff-error">
          <summary>Could not load changes</summary>
          <pre>{diffError}</pre>
        </details>
      )}
      {!diffLoading && !diffs.length && !diffError && (
        <EmptyState
          compact
          kind="completed"
          title={isRepository ? "No applied changes" : "Not a Git repository"}
          description={
            isRepository
              ? "There are no file changes for this comparison."
              : "Chat, Terminal and Files are available. To use Git review, open a project that has a Git repository."
          }
        />
      )}
      {diffs.map((file) => (
        <div className="git-file" key={file.new_path || file.old_path}>
          <div className="diff-file-header">
            <FileCode2 size={14} />
            <span>{file.new_path || file.old_path}</span>
          </div>
          {!file.hunks.length && (
            <p className="live-hint">
              No text preview available for this empty, binary or large file.
            </p>
          )}
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
        {paletteMode && (
          <CommandPalette
            mode={paletteMode}
            commands={paletteCommands}
            onClose={closePalette}
          />
        )}
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
            <div
              className="session-view-controls"
              role="group"
              aria-label="Session view"
            >
              <button
                aria-pressed={sessionView === "activity"}
                onClick={() => setSessionView("activity")}
              >
                Activity
              </button>
              <button
                aria-pressed={sessionView === "projects"}
                onClick={() => setSessionView("projects")}
              >
                Projects
              </button>
              <button
                aria-pressed={showArchived}
                onClick={() => setShowArchived((v) => !v)}
              >
                Archived ({sessions.filter((s) => isArchived(s.id)).length})
              </button>
              <button
                aria-label="Compact session rows"
                title="Compact session rows"
                aria-pressed={density === "compact"}
                onClick={() =>
                  setDensity((v) =>
                    v === "compact" ? "comfortable" : "compact",
                  )
                }
              >
                ≡
              </button>
            </div>
            <SessionList
              density={density}
              groups={groups}
              singleLineCompact
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
                        await backend.request(
                          managed ? "/api/chat" : "/api/interrupt",
                          {
                            session_id: session.id,
                            ...(managed ? { operation: "cancel" } : {}),
                          },
                        );
                        setNotice("Interrupt sent to this session");
                      })
                    }
                  >
                    Interrupt
                  </button>
                  {!alive && (
                    <button className="live-action" onClick={archiveSession}>
                      {isArchived(session.id)
                        ? "Restore from archive"
                        : "Archive session"}
                    </button>
                  )}
                  <button
                    className="live-action"
                    disabled={!alive || !connected}
                    onClick={() =>
                      void act(async () => {
                        await backend.request("/api/freeze", {
                          session_id: session.id,
                        });
                        setNotice("Session ended · moved to Inactive");
                        await refresh();
                      })
                    }
                  >
                    End session
                  </button>
                </>
              )}
              <button
                className="live-action"
                aria-label="Open command palette"
                title="Command palette (Ctrl/⌘ Shift P)"
                onClick={() => openPalette("commands")}
              >
                <Search size={14} /> Commands
              </button>
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
              <div>
                <span>{error.split("\n")[0].slice(0, 220)}</span>
                {error.length > 220 || error.includes("\n") ? (
                  <details>
                    <summary>Error details</summary>
                    <pre>{error}</pre>
                  </details>
                ) : null}
              </div>
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
                  description="Launch an installed coding agent. Sessions run in the Rust daemon and survive closing this tab."
                  action="Start your first session"
                  onAction={() => setModal("new")}
                />
              ) : (
                <>
                  {managed && (
                    <div
                      className="managed-conversation-host"
                      style={{
                        display:
                          tab === "Chat" || tab === "Terminal"
                            ? "flex"
                            : "none",
                      }}
                    >
                      <GeminiChat
                        key={session.id}
                        backend={backend}
                        sessionId={session.id}
                        connected={connected && alive}
                        onSnapshot={(state) =>
                          setReviewChats((previous) => ({
                            ...previous,
                            [session.id]: state,
                          }))
                        }
                        presentation={tab === "Terminal" ? "terminal" : "chat"}
                        draft={drafts[session.id] || ""}
                        onDraft={(text) =>
                          setDrafts((previous) => ({
                            ...previous,
                            [session.id]: text,
                          }))
                        }
                        onStatus={(status) =>
                          setChatStatuses((previous) =>
                            previous[session.id] === status
                              ? previous
                              : { ...previous, [session.id]: status },
                          )
                        }
                      />
                    </div>
                  )}
                  {tab === "Chat" && !managed && (
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
                  {tab === "Terminal" && !managed && (
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
                        {agents.map((agent) => (
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
                {error && (
                  <div className="live-error" role="alert">
                    <div>
                      <span>{error.split("\n")[0].slice(0, 220)}</span>
                      <details>
                        <summary>Error details</summary>
                        <pre>{error}</pre>
                      </details>
                    </div>
                  </div>
                )}
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
                        {!newAgent && (
                          <option value="">
                            Install an agent to get started
                          </option>
                        )}
                        {agents.map((agent) => (
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
                    {!agents.some((a) => a.available) && (
                      <p className="live-hint" role="status">
                        No agent provider is installed. Install Gemini CLI,
                        Claude Code, Codex or another supported agent, then
                        refresh installed agents.{" "}
                        <button
                          type="button"
                          className="live-action"
                          onClick={() =>
                            void act(async () =>
                              setInfo(
                                await backend.request<MachineInfo>("/api/info"),
                              ),
                            )
                          }
                        >
                          Refresh providers
                        </button>
                      </p>
                    )}
                    {newAgent === "gemini" && (
                      <div className="gemini-provider-setup">
                        <p className="live-hint">
                          Gemini runs locally. Sign in through the CLI; Chat
                          displays tool approval requests in your conversation.
                        </p>
                        {!info?.agents.find((agent) => agent.id === "gemini")
                          ?.available && (
                          <p role="status">
                            Install Node.js 20+ and run{" "}
                            <code>npm install -g @google/gemini-cli</code>, then
                            restart Agmux if PATH changed.
                          </p>
                        )}
                        <button
                          type="button"
                          className="live-action"
                          onClick={() =>
                            void act(async () =>
                              setInfo(
                                await backend.request<MachineInfo>("/api/info"),
                              ),
                            )
                          }
                        >
                          Refresh installed agents
                        </button>
                        <label>
                          Model
                          <select
                            aria-label="Gemini model"
                            value={geminiModel}
                            onChange={(event) =>
                              setGeminiModel(event.target.value)
                            }
                          >
                            <option value="">
                              CLI configured default
                              {models?.currentModelId
                                ? ` (${models.currentModelId})`
                                : ""}
                            </option>
                            {models?.availableModels?.map((model) => (
                              <option key={model.modelId} value={model.modelId}>
                                {model.name || model.modelId}
                              </option>
                            ))}
                          </select>
                        </label>
                        {modelsLoading && (
                          <p className="live-hint" role="status">
                            Discovering models from Gemini CLI…
                          </p>
                        )}
                        {modelsError && (
                          <details className="model-discovery-error">
                            <summary>Using CLI configured default</summary>
                            <p>{modelsError}</p>
                            <button
                              type="button"
                              className="live-action"
                              onClick={() => setModelRetry((n) => n + 1)}
                            >
                              Retry model discovery
                            </button>
                          </details>
                        )}
                        <p className="live-hint">
                          Use the configured default or a model advertised by
                          your CLI. Shared Gemini sessions accept the Model
                          setting. Other CLI arguments are available for custom
                          executables. No API key is stored in this UI.
                        </p>
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
                    <details className="advanced-settings">
                      <summary>Advanced settings</summary>
                      <label>
                        Arguments (JSON array)
                        <input
                          value={args}
                          onChange={(event) => setArgs(event.target.value)}
                          placeholder={'["--flag"]'}
                        />
                      </label>
                    </details>
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
                        disabled={
                          busy ||
                          !connected ||
                          !info ||
                          !newAgent ||
                          (newAgent !== "custom" &&
                            !info?.agents.find((agent) => agent.id === newAgent)
                              ?.available)
                        }
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
