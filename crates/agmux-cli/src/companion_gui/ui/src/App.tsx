import {
  useEffect,
  useRef,
  useMemo,
  useState,
  lazy,
  Suspense,
  type ReactNode,
} from "react";
import {
  AnimatePresence,
  motion,
  MotionConfig,
  useReducedMotion,
} from "motion/react";
import {
  AnimatedPopover,
  AnimatedOverlay,
  AnimatedCheck,
  Selection,
  useViewportWidth,
} from "./MotionUI";
import { motionTiming, sessionDirection } from "./motion-spec";
import { Pressable, Text } from "react-native-web";
import {
  ArrowLeft,
  ArrowRight,
  Bell,
  Bot,
  Check,
  ChevronDown,
  CircleCheck,
  Command,
  Copy,
  Clock3,
  Ellipsis,
  FileCode2,
  Folder,
  GitBranch,
  GitCompareArrows,
  Layers,
  Maximize2,
  Scan,
  Minimize2,
  Menu,
  MessageSquare,
  Monitor,
  PanelLeft,
  PanelRight,
  Pin,
  Plus,
  Search,
  Settings,
  ShieldCheck,
  SquareTerminal,
  TerminalSquare,
  X,
} from "lucide-react";
import {
  files,
  initialSessions,
  source,
  type Agent,
  type Message,
  type Session,
} from "./demo";
import Composer, { agents, type Attachment } from "./Composer";
import {
  SessionList,
  Transcript,
  PanelDivider,
  type TranscriptHandle,
} from "./WorkspaceLists";
import {
  statuses,
  nextSessionId,
  moveQueuedMessage,
  type QueuedMessage,
  type Layout,
} from "./workspace";
import AgentIcon from "./AgentAvatar";
import EmptyState from "./EmptyState";
import ConnectBackend from "./ConnectBackend";
import { Diff } from "./CodeViews";
const CodeEditor = lazy(() => import("./CodeEditor"));
const Terminal = lazy(() => import("./Terminal"));

type Tab = "Chat" | "Files" | "Git" | "Terminal" | "Machines";
function IconButton({
  label,
  children,
  onClick,
  active = false,
  disabled = false,
}: {
  label: string;
  children: ReactNode;
  onClick?: () => void;
  active?: boolean;
  disabled?: boolean;
}) {
  return (
    <motion.button
      whileTap={{ scale: 0.93 }}
      type="button"
      disabled={disabled}
      title={label}
      aria-label={label}
      className={"icon-button" + (active ? " active" : "")}
      onClick={onClick}
    >
      {children}
    </motion.button>
  );
}
function Action({
  children,
  onPress,
  primary = false,
  label,
}: {
  children: ReactNode;
  onPress: () => void;
  primary?: boolean;
  label?: string;
}) {
  return (
    <Pressable
      {...{ role: "button", "aria-label": label }}
      onPress={onPress}
      style={({ pressed }) => ({
        flexDirection: "row",
        gap: 7,
        alignItems: "center",
        justifyContent: "center",
        borderRadius: 7,
        paddingHorizontal: 12,
        paddingVertical: 8,
        backgroundColor: primary ? "var(--button-bg)" : "var(--surface-raised)",
        opacity: pressed ? 0.7 : 1,
        borderWidth: 1,
        borderColor: primary ? "var(--button-bg)" : "var(--line)",
      })}
    >
      <Text
        style={{
          color: primary ? "var(--button-text)" : "var(--text)",
          fontSize: 12,
          fontWeight: "600",
        }}
      >
        {children}
      </Text>
    </Pressable>
  );
}
function ToolCard({ title, lines }: { title: string; lines: string[] }) {
  const [expanded, setExpanded] = useState(false);
  const reduced = !!useReducedMotion();
  return (
    <div className="tool-card" aria-label={title}>
      <button
        className="tool-title"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
      >
        <Layers size={14} />
        <span>
          {title}
          <small>
            {lines.length}{" "}
            {lines[0]?.startsWith("src/") ? "files updated" : "commands"}
          </small>
        </span>
        <motion.span
          className="disclosure-chevron"
          animate={{ rotate: expanded ? 0 : -90 }}
          transition={motionTiming(reduced, "control")}
        >
          <ChevronDown size={13} />
        </motion.span>
        <CircleCheck size={13} className="success" />
      </button>
      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            className="tool-expander"
            key="details"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={motionTiming(reduced, "disclosure")}
          >
            <div className="tool-lines">
              {lines.map((line) => (
                <div key={line}>
                  {line.startsWith("src/") ? (
                    <FileCode2 size={14} />
                  ) : (
                    <TerminalSquare size={14} />
                  )}
                  <strong>
                    {line.startsWith("src/") ? "Edited" : "Terminal"}
                  </strong>
                  <span>{line}</span>
                  <Check size={12} />
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
function readSaved<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) || "null") ?? fallback;
  } catch {
    return fallback;
  }
}

export default function App() {
  const reduced = !!useReducedMotion();
  const viewportWidth = useViewportWidth();
  const sidebarWidth =
    viewportWidth < 700
      ? Math.min(310, viewportWidth * 0.9)
      : viewportWidth <= 1200
        ? 260
        : 290;
  const [navigationDirection, setNavigationDirection] = useState(1);
  const [resizing, setResizing] = useState(false);
  const [toastVersion, setToastVersion] = useState(0);
  const menuCloseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (menuCloseTimer.current) clearTimeout(menuCloseTimer.current);
    },
    [],
  );
  const [sessions, setSessions] = useState<Session[]>(() =>
    readSaved("agmux-demo-sessions", initialSessions),
  );
  const [selected, setSelected] = useState("workspace");
  const session: Session = sessions.find((item) => item.id === selected) ||
    sessions[0] || {
      id: "welcome",
      title: "Your workspace",
      agent: "Claude Code",
      project: "No project selected",
      state: "idle",
      time: "now",
    };
  const [tab, setTab] = useState<Tab>("Chat");
  const [sidebar, setSidebar] = useState(
    () =>
      window.innerWidth >= 700 &&
      readSaved<Layout>("agmux-ui-layout", {
        sidebar: true,
        review: true,
        reviewWidth: 420,
      }).sidebar,
  );
  const [reviewWidth, setReviewWidth] = useState(
    () =>
      readSaved<Layout>("agmux-ui-layout", {
        sidebar: true,
        review: true,
        reviewWidth: 420,
      }).reviewWidth,
  );
  const [recentIds, setRecentIds] = useState<string[]>(() =>
    readSaved("agmux-recent-sessions", ["workspace"]),
  );
  const [agentSearch, setAgentSearch] = useState("");
  const [review, setReview] = useState(
    () =>
      window.innerWidth >= 1050 &&
      readSaved<Layout>("agmux-ui-layout", {
        sidebar: true,
        review: true,
        reviewWidth: 420,
      }).review,
  );
  const [wideReview, setWideReview] = useState(
    () =>
      !!readSaved<Layout>("agmux-ui-layout", {
        sidebar: true,
        review: true,
        reviewWidth: 420,
      }).wideReview,
  );
  const [search, setSearch] = useState("");
  const [agentFilter, setAgentFilter] = useState<Agent | "All agents">(
    "All agents",
  );
  const [drafts, setDrafts] = useState<Record<string, string>>(() =>
    readSaved("agmux-demo-drafts", {}),
  );
  const [messages, setMessages] = useState<Record<string, Message[]>>(() =>
    readSaved("agmux-demo-messages", {}),
  );
  const [running, setRunning] = useState<Record<string, boolean>>({});
  const [approved, setApproved] = useState<
    Record<string, "allowed" | "denied">
  >({});
  const [modal, setModal] = useState<"new" | "settings" | "commands" | "connect" | null>(
    null,
  );
  const [newTitle, setNewTitle] = useState("");
  const [newAgent, setNewAgent] = useState<Agent>("Claude Code");
  const [newProject, setNewProject] = useState("/workspace/agmux");
  const [menu, setMenu] = useState<
    "agent" | "mode" | "more" | "filter" | "recent" | "status" | null
  >(null);
  const [mode, setMode] = useState("Default");
  const [file, setFile] = useState(0);
  const [reviewScope, setReviewScope] = useState("Included");
  const [reviewed, setReviewed] = useState(false);
  const [editor, setEditor] = useState(
    () =>
      readSaved<Record<string, string>>("agmux-demo-files", {})[
        files[0].path
      ] || source,
  );
  const [documents, setDocuments] = useState<Record<string, string>>(() =>
    readSaved("agmux-demo-files", {}),
  );
  const [diffLayout, setDiffLayout] = useState<"unified" | "split">("unified");
  const [machineOffline, setMachineOffline] = useState(false);
  const [toast, setToast] = useState("");
  const [attachments, setAttachments] = useState<Record<string, Attachment[]>>(
    {},
  );
  const [queues, setQueues] = useState<Record<string, QueuedMessage[]>>(() =>
    readSaved("agmux-message-queues", {}),
  );
  const queuesRef = useRef(queues);
  const objectURLs = useRef(new Set<string>());
  const [command, setCommand] = useState("");
  const [commandIndex, setCommandIndex] = useState(0);
  const [light, setLight] = useState(
    () =>
      readSaved("agmux-appearance", { light: false, density: "comfortable" })
        .light,
  );
  const [density, setDensity] = useState<"comfortable" | "compact">(
    () =>
      readSaved("agmux-appearance", { light: false, density: "comfortable" })
        .density as "comfortable" | "compact",
  );
  const [focusView, setFocusView] = useState(false);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const availableWidth =
    viewportWidth - (sidebar && !focusView ? sidebarWidth : 0);
  const reviewDockWidth =
    viewportWidth < 1050
      ? Math.min(480, viewportWidth * 0.94)
      : Math.max(
          280,
          Math.min(
            wideReview ? availableWidth * 0.57 : reviewWidth,
            availableWidth - 340,
          ),
        ) + 7;
  const transcript = useRef<TranscriptHandle>(null);
  const scrollPositions = useRef<Record<string, number>>({});
  const timers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const draft = drafts[session.id] || "";
  const notify = (message: string) => {
    setToast(message);
    setToastVersion((value) => value + 1);
  };
  useEffect(() => {
    if (focusView && tab !== "Chat") setFocusView(false);
  }, [tab, focusView]);
  const toggleFocus = () => {
    setFocusView((value) => !value);
    setTab("Chat");
    setMenu(null);
  };
  useEffect(() => {
    localStorage.setItem(
      "agmux-appearance",
      JSON.stringify({ light, density }),
    );
    document.documentElement.style.colorScheme = light ? "light" : "dark";
  }, [light, density]);
  const choose = (id: string, direction?: -1 | 1) => {
    setNavigationDirection(
      sessionDirection(
        sessions.map((item) => item.id),
        session.id,
        id,
        direction,
      ),
    );
    setSelected(id);
    setRecentIds((prev) =>
      [id, ...prev.filter((value) => value !== id)].slice(0, 12),
    );
    setMenu(null);
    if (window.innerWidth < 850) setSidebar(false);
  };
  useEffect(() => {
    localStorage.setItem("agmux-demo-sessions", JSON.stringify(sessions));
  }, [sessions]);
  useEffect(() => {
    localStorage.setItem("agmux-demo-drafts", JSON.stringify(drafts));
  }, [drafts]);
  useEffect(() => {
    const timer = setTimeout(() => {
      try {
        localStorage.setItem("agmux-demo-messages", JSON.stringify(messages));
      } catch {
        setToast(
          "Browser storage is full · this conversation is available until you close the page",
        );
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [messages]);
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 3200);
    return () => clearTimeout(id);
  }, [toast]);
  useEffect(
    () => () => {
      Object.values(timers.current).forEach(clearTimeout);
      objectURLs.current.forEach(URL.revokeObjectURL);
    },
    [],
  );
  useEffect(() => {
    localStorage.setItem(
      "agmux-ui-layout",
      JSON.stringify({ sidebar, review, reviewWidth, wideReview }),
    );
  }, [sidebar, review, reviewWidth, wideReview]);
  useEffect(() => {
    localStorage.setItem("agmux-recent-sessions", JSON.stringify(recentIds));
  }, [recentIds]);
  useEffect(() => {
    localStorage.setItem("agmux-message-queues", JSON.stringify(queues));
  }, [queues]);
  useEffect(() => {
    if (!modal) return;
    const dialog = document.querySelector<HTMLElement>("[role=dialog]");
    const focusable = () =>
      Array.from(
        dialog?.querySelectorAll<HTMLElement>(
          'button, input, select, textarea, [tabindex="0"]',
        ) || [],
      );
    if (dialog && !dialog.contains(document.activeElement))
      focusable()[0]?.focus();
    const trap = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const items = focusable();
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    document.addEventListener("keydown", trap);
    return () => document.removeEventListener("keydown", trap);
  }, [modal]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setModal((value) => (value === "commands" ? null : "commands"));
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setModal("new");
      }
      if (
        !modal &&
        event.altKey &&
        (event.key === "ArrowUp" || event.key === "ArrowDown")
      ) {
        event.preventDefault();
        choose(
          nextSessionId(sessions, session.id, event.key === "ArrowUp" ? -1 : 1),
          event.key === "ArrowUp" ? -1 : 1,
        );
      }
      if (!modal && event.altKey && event.key.toLowerCase() === "a") {
        event.preventDefault();
        setMenu((value) => (value === "agent" ? null : "agent"));
        setAgentSearch("");
      }
      if (!modal && event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        toggleFocus();
      }
      if (event.key === "Escape") {
        if (!modal) setFocusView(false);
        setModal(null);
        setMenu(null);
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [sessions, session.id, modal]);
  const updateQueue = (
    id: string,
    update: (queue: QueuedMessage[]) => QueuedMessage[],
  ) => {
    const next = {
      ...queuesRef.current,
      [id]: update(queuesRef.current[id] || []),
    };
    queuesRef.current = next;
    setQueues(next);
  };
  const stopTurn = (id: string, announce = true) => {
    clearTimeout(timers.current[id]);
    delete timers.current[id];
    setRunning((prev) => ({ ...prev, [id]: false }));
    setSessions((prev) =>
      prev.map((item) =>
        item.id === id ? { ...item, state: "stopped" } : item,
      ),
    );
    if (announce) notify("Response stopped · queued messages are paused");
  };
  const startTurn = (
    id: string,
    text: string,
    names: string[] = [],
    messageId?: string,
  ): void => {
    const agent = sessions.find((item) => item.id === id)?.agent || "Agent";
    const reply = `Your message is saved in this demo session. In the connected application, ${agent} will receive it through the agmux companion bridge.\n\nYou can switch sessions, review the changes, or open the terminal while keeping this conversation.`;
    const responseId = crypto.randomUUID();
    if (messageId) notify("Queued prompt sent · preview");
    setMessages((prev) => ({
      ...prev,
      [id]: [
        ...(prev[id] || []),
        {
          id: messageId || crypto.randomUUID(),
          role: "user",
          text: text + names.map((name) => `\n📎 ${name}`).join(""),
        },
      ],
    }));
    setRunning((prev) => ({ ...prev, [id]: true }));
    setSessions((prev) =>
      prev.map((item) =>
        item.id === id ? { ...item, state: "working", time: "now" } : item,
      ),
    );
    timers.current[id] = setTimeout(() => {
      setMessages((prev) => ({
        ...prev,
        [id]: [
          ...(prev[id] || []),
          { id: responseId, role: "assistant", text: "" },
        ],
      }));
      let cursor = 0;
      const tick = () => {
        cursor += 22;
        setMessages((prev) => ({
          ...prev,
          [id]: (prev[id] || []).map((message) =>
            message.id === responseId
              ? { ...message, text: reply.slice(0, cursor) }
              : message,
          ),
        }));
        if (cursor < reply.length) timers.current[id] = setTimeout(tick, 65);
        else {
          delete timers.current[id];
          setRunning((prev) => ({ ...prev, [id]: false }));
          setSessions((prev) =>
            prev.map((item) =>
              item.id === id ? { ...item, state: "idle" } : item,
            ),
          );
          const next = queuesRef.current[id]?.[0];
          if (next) {
            updateQueue(id, (queue) => queue.slice(1));
            startTurn(id, next.text, next.attachments, next.id);
          }
        }
      };
      tick();
    }, 500);
  };
  const send = () => {
    if (!sessions.length || !draft.trim() || session.state === "reconnecting")
      return;
    const id = session.id;
    const names = (attachments[id] || []).map((item) => item.name);
    if (running[id]) {
      updateQueue(id, (queue) => [
        ...queue,
        { id: crypto.randomUUID(), text: draft.trim(), attachments: names },
      ]);
    } else {
      startTurn(id, draft.trim(), names);
      requestAnimationFrame(() => transcript.current?.jumpToLatest());
    }
    setDrafts((prev) => ({ ...prev, [id]: "" }));
    (attachments[id] || []).forEach((item) => {
      if (item.url) {
        URL.revokeObjectURL(item.url);
        objectURLs.current.delete(item.url);
      }
    });
    setAttachments((prev) => ({ ...prev, [id]: [] }));
  };
  const addAttachments = async (incoming: File[]) => {
    const id = session.id;
    const accepted = incoming
      .filter((file) => file.size <= 10 * 1024 * 1024)
      .slice(0, 8);
    if (accepted.length !== incoming.length)
      notify("Choose up to 8 files, each under 10 MB");
    const additions = await Promise.all(
      accepted.map(async (file) => {
        const item: Attachment = {
          id: crypto.randomUUID(),
          name: file.name,
          size: file.size,
          type: file.type,
        };
        if (file.type.startsWith("image/") && file.type !== "image/svg+xml") {
          item.url = URL.createObjectURL(file);
          objectURLs.current.add(item.url);
        } else if (
          (file.type.startsWith("text/") ||
            /\.(md|txt|json|rs|tsx?|css)$/.test(file.name)) &&
          file.size < 128 * 1024
        )
          item.excerpt = (await file.text()).slice(0, 180);
        return item;
      }),
    );
    setAttachments((prev) => {
      const merged = [...(prev[id] || []), ...additions];
      const overflow = merged.slice(8);
      overflow.forEach((item) => {
        if (item.url) {
          URL.revokeObjectURL(item.url);
          objectURLs.current.delete(item.url);
        }
      });
      return { ...prev, [id]: merged.slice(0, 8) };
    });
  };
  const create = () => {
    if (!newTitle.trim() || !newProject.trim()) return;
    const id = crypto.randomUUID();
    setSessions((prev) => [
      ...prev,
      {
        id,
        title: newTitle.trim(),
        agent: newAgent,
        project:
          newProject.trim().split("/").filter(Boolean).pop() ||
          newProject.trim(),
        state: "idle",
        time: "now",
      },
    ]);
    choose(id);
    setModal(null);
    setNewTitle("");
    setTab("Chat");
    notify("Demo session created");
  };
  const filtered = useMemo(
    () =>
      sessions.filter(
        (item) =>
          item.title.toLowerCase().includes(search.toLowerCase()) &&
          (agentFilter === "All agents" || item.agent === agentFilter),
      ),
    [sessions, search, agentFilter],
  );
  const groups = [
    {
      name: "Needs attention",
      items: filtered.filter(
        (item) =>
          (item.state === "attention" || item.state === "reconnecting") &&
          !item.pinned,
      ),
    },
    {
      name: "Working",
      items: filtered.filter(
        (item) => item.state === "working" && !item.pinned,
      ),
    },
    { name: "Pinned", items: filtered.filter((item) => item.pinned) },
    {
      name: "Recent",
      items: filtered.filter((item) => item.state === "idle" && !item.pinned),
    },
  ];
  const commandItems = [
    {
      name: "Start new session",
      icon: <Plus size={17} />,
      action: () => setModal("new"),
      key: "⌘ N",
    },
    ...(["Chat", "Files", "Git", "Terminal"] as Tab[]).map((name) => ({
      name: `Open ${name.toLowerCase()}`,
      icon: <PanelRight size={17} />,
      action: () => {
        setTab(name);
        setModal(null);
      },
      key: "",
    })),
    {
      name: "Toggle review panel",
      icon: <GitCompareArrows size={17} />,
      action: () => {
        setReview(!review);
        setModal(null);
      },
      key: "",
    },
    ...sessions.map((item) => ({
      name: item.title,
      icon: <AgentIcon agent={item.agent} />,
      action: () => {
        choose(item.id);
        setModal(null);
      },
      key: item.agent,
    })),
  ].filter((item) => item.name.toLowerCase().includes(command.toLowerCase()));
  return (
    <MotionConfig reducedMotion="user">
      <div
        className={
          "app" + (light ? " light" : "") + (focusView ? " focus-view" : "")
        }
        data-theme={light ? "light" : "dark"}
        data-density={density}
      >
        <AnimatePresence initial={false}>
          {sidebar && !focusView && (
            <motion.aside
              key="sidebar"
              className="sidebar motion-sidebar"
              initial={{ width: 0, opacity: 0 }}
              animate={{ width: sidebarWidth, opacity: 1 }}
              exit={{ width: 0, opacity: 0 }}
              transition={motionTiming(reduced, "panel")}
            >
              <div className="sidebar-content" style={{ width: sidebarWidth }}>
                <div className="sidebar-chrome">
                  <div className="traffic">
                    <i />
                    <i />
                    <i />
                  </div>
                  <div className="row">
                    <IconButton
                      label="Search workspace"
                      onClick={() => {
                        setModal("commands");
                        setCommand("");
                      }}
                    >
                      <Search size={14} />
                    </IconButton>
                    <IconButton
                      label="Hide sidebar"
                      onClick={() => setSidebar(false)}
                    >
                      <PanelLeft size={14} />
                    </IconButton>
                  </div>
                </div>
                <div className="brand-row">
                  <div className="brand-mark">
                    <span>⌘</span>
                  </div>
                  <div className="brand-copy">
                    <div>
                      <strong>Agmux</strong>
                      <span className="tiny-badge">LOCAL</span>
                    </div>
                    <span>
                      <i className="status-dot green" /> Your agent workspace
                    </span>
                  </div>
                  <IconButton
                    label="New session"
                    onClick={() => setModal("new")}
                  >
                    <Plus size={21} />
                  </IconButton>
                </div>
                <div className="sidebar-switch">
                  <button
                    className={tab !== "Machines" ? "selected" : ""}
                    onClick={() => {
                      setTab("Chat");
                      setAgentFilter("All agents");
                      setSearch("");
                    }}
                  >
                    <Selection
                      active={tab !== "Machines"}
                      id="sidebar-view-selection"
                    />
                    Sessions <span>{sessions.length}</span>
                  </button>
                  <button
                    className={tab === "Machines" ? "selected" : ""}
                    onClick={() => setTab("Machines")}
                  >
                    <Selection
                      active={tab === "Machines"}
                      id="sidebar-view-selection"
                    />
                    Machines <span>1</span>
                  </button>
                </div>
                <div className="search-box">
                  <Search size={14} />
                  <input
                    aria-label="Search sessions"
                    placeholder="Search sessions…"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                  <kbd>⌘ K</kbd>
                </div>
                <SessionList
                  density={density}
                  onClear={() => {
                    setSearch("");
                    setAgentFilter("All agents");
                  }}
                  groups={groups}
                  collapsed={collapsed}
                  selected={session.id}
                  onSelect={choose}
                  onCollapse={(name) =>
                    setCollapsed((prev) => ({ ...prev, [name]: !prev[name] }))
                  }
                  onFilter={() => setMenu(menu === "filter" ? null : "filter")}
                  icon={(item) => <AgentIcon agent={item.agent} />}
                />
                <AnimatePresence initial={false}>
                  {menu === "filter" && (
                    <AnimatedPopover className="filter-popover popover">
                      <span className="menu-label">Filter by agent</span>
                      {["All agents", ...agents].map((name) => (
                        <button
                          key={name}
                          onClick={() => {
                            setAgentFilter(name as Agent | "All agents");
                            setMenu(null);
                          }}
                        >
                          {name}
                          {agentFilter === name && <Check size={13} />}
                        </button>
                      ))}
                    </AnimatedPopover>
                  )}
                </AnimatePresence>
                <div className="sidebar-footer">
                  <button
                    className="new-session"
                    onClick={() => setModal("new")}
                  >
                    <Plus size={16} />
                    Start new session<kbd>⌘ N</kbd>
                  </button>
                  <div className="footer-links">
                    <button onClick={() => setModal("settings")}>
                      <Monitor size={14} />
                      <i className="status-dot green" />
                      Local machine
                      <ChevronDown size={11} />
                    </button>
                    <IconButton
                      label="Settings"
                      onClick={() => setModal("settings")}
                    >
                      <Settings size={16} />
                    </IconButton>
                  </div>
                </div>
              </div>
            </motion.aside>
          )}
        </AnimatePresence>
        <main className="workspace">
          <header className="workspace-header">
            <div className="row header-title">
              {!sidebar && !focusView && (
                <IconButton
                  label="Show sidebar"
                  onClick={() => setSidebar(true)}
                >
                  <Menu size={18} />
                </IconButton>
              )}
              <IconButton
                label="Previous session"
                disabled={!sessions.length}
                onClick={() =>
                  choose(nextSessionId(sessions, session.id, -1), -1)
                }
              >
                <ArrowLeft size={19} />
              </IconButton>
              <IconButton
                label="Recent sessions"
                onClick={() => setMenu(menu === "recent" ? null : "recent")}
              >
                <Clock3 size={16} />
              </IconButton>
              <div>
                <h1>{session.title}</h1>
                <span className="workspace-subtitle">
                  <Folder size={11} />
                  {session.project}
                  <span> / </span>
                  <GitBranch size={11} />
                  companion-ui
                </span>
              </div>
              <button
                className={"header-session-state " + session.state}
                onClick={() => setMenu(menu === "status" ? null : "status")}
                title="Preview session states"
                aria-label="Preview session status"
              >
                <i className={"status-dot " + statuses[session.state].color} />
                {statuses[session.state].label}
                <ChevronDown size={10} />
              </button>
            </div>
            <div className="row header-actions"><button className="live-action connect-trigger" onClick={() => setModal("connect")}><Monitor size={14}/><span>Connect backend</span></button>
              <button
                className="focus-toggle"
                aria-label={focusView ? "Exit focus view" : "Enter focus view"}
                aria-pressed={focusView}
                onClick={toggleFocus}
              >
                {focusView ? <Minimize2 size={15} /> : <Scan size={15} />}
                <span>{focusView ? "Exit focus" : "Focus"}</span>
              </button>
              <IconButton
                label="Session actions"
                onClick={() => setMenu(menu === "more" ? null : "more")}
              >
                <Ellipsis size={20} />
              </IconButton>
              <IconButton
                label="Choose agent"
                onClick={() => setMenu(menu === "agent" ? null : "agent")}
              >
                <Bot size={18} />
              </IconButton>
              <IconButton
                label="Open terminal"
                disabled={!sessions.length}
                active={tab === "Terminal"}
                onClick={() => setTab("Terminal")}
              >
                <SquareTerminal size={18} />
              </IconButton>
              <IconButton
                label="Toggle review panel"
                active={review}
                onClick={() => setReview(!review)}
              >
                <PanelRight size={17} />
              </IconButton>
              <button
                className="avatar"
                onClick={() => setModal("settings")}
                aria-label="Workspace settings"
              >
                M
              </button>
            </div>
          </header>
          <AnimatePresence initial={false}>
            {menu === "recent" && (
              <AnimatedPopover className="recent-popover popover">
                <span className="menu-label">
                  Recent sessions <kbd>Alt ↑ / ↓</kbd>
                </span>
                {recentIds
                  .map((id) => sessions.find((item) => item.id === id))
                  .filter((item): item is Session => !!item)
                  .map((item) => (
                    <button key={item.id} onClick={() => choose(item.id)}>
                      <AgentIcon agent={item.agent} />
                      <span>{item.title}</span>
                      {session.id === item.id && <Check size={13} />}
                    </button>
                  ))}
              </AnimatedPopover>
            )}
          </AnimatePresence>
          <AnimatePresence initial={false}>
            {menu === "status" && (
              <AnimatedPopover className="status-popover popover">
                <span className="menu-label">Preview states · simulated</span>
                {(Object.keys(statuses) as Session["state"][]).map((state) => (
                  <button
                    key={state}
                    onClick={() => {
                      if (running[session.id]) stopTurn(session.id, false);
                      setSessions((prev) =>
                        prev.map((item) =>
                          item.id === session.id ? { ...item, state } : item,
                        ),
                      );
                      setMenu(null);
                    }}
                  >
                    <i className={"status-dot " + statuses[state].color} />
                    {statuses[state].label}
                    {session.state === state && <Check size={13} />}
                  </button>
                ))}
              </AnimatedPopover>
            )}
          </AnimatePresence>
          <AnimatePresence initial={false}>
            {menu === "more" && (
              <AnimatedPopover className="session-popover popover">
                <button onClick={() => setMenu("status")}>
                  <Monitor size={14} />
                  Preview connection state
                </button>
                <button
                  onClick={() => {
                    setSessions((prev) =>
                      prev.map((item) =>
                        item.id === session.id
                          ? { ...item, pinned: !item.pinned }
                          : item,
                      ),
                    );
                    setMenu(null);
                  }}
                >
                  <Pin size={14} />
                  {session.pinned ? "Unpin session" : "Pin session"}
                </button>
                <button
                  onClick={() => {
                    navigator.clipboard
                      ?.writeText(`agmux://session/${session.id}`)
                      .then(() => notify("Session link copied"))
                      .catch(() => notify("Clipboard unavailable"));
                    setMenu(null);
                  }}
                >
                  <Copy size={14} />
                  Copy session link
                </button>
                <button
                  onClick={() => {
                    setModal("new");
                    setMenu(null);
                  }}
                >
                  <Plus size={14} />
                  New session
                </button>
              </AnimatedPopover>
            )}
          </AnimatePresence>
          <div className="workspace-body">
            <section
              className={"conversation-pane" + (wideReview ? " reduced" : "")}
            >
              <nav className="view-tabs" aria-label="Workspace views">
                {(
                  [
                    { name: "Chat", icon: <MessageSquare size={14} /> },
                    { name: "Files", icon: <FileCode2 size={14} /> },
                    { name: "Git", icon: <GitBranch size={14} /> },
                    { name: "Terminal", icon: <SquareTerminal size={14} /> },
                  ] as const
                ).map((item) => (
                  <button
                    key={item.name}
                    disabled={!sessions.length}
                    onClick={() => setTab(item.name)}
                    className={tab === item.name ? "active" : ""}
                    aria-current={tab === item.name ? "page" : undefined}
                  >
                    {tab === item.name && (
                      <motion.span
                        layoutId="workspace-tab-indicator"
                        className="active-tab-indicator"
                        transition={motionTiming(reduced, "navigation")}
                      />
                    )}
                    {item.icon}
                    {item.name}
                    {item.name === "Git" && <span>3</span>}
                  </button>
                ))}
                <span className="tab-end">
                  <i className="status-dot green" />
                  Local demo
                </span>
              </nav>
              {tab === "Chat" && (
                <>
                  <motion.div
                    className="session-stage"
                    key={session.id}
                    initial={{
                      opacity: 0,
                      x: reduced ? 0 : navigationDirection * 8,
                    }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={motionTiming(reduced, "navigation")}
                  >
                    {!sessions.length ? (
                      <EmptyState
                        kind="workspace"
                        title="Your next idea starts here"
                        description="Give your agents a place to work. Start a session, choose a project, and keep every conversation close."
                        action="Start your first session"
                        onAction={() => setModal("new")}
                      />
                    ) : (
                      <Transcript
                        density={density}
                        key={session.id}
                        ref={transcript}
                        sessionId={session.id}
                        agent={session.agent}
                        messages={messages[session.id] || []}
                        running={!!running[session.id]}
                        onStop={() => stopTurn(session.id)}
                        positions={scrollPositions}
                        intro={
                          <>
                            <div className="day-divider">
                              <span>Today</span>
                            </div>
                            {session.id === "workspace" ? (
                              <>
                                <div className="user-message">
                                  <div className="message-author">
                                    You <span>12:38 PM</span>
                                  </div>
                                  <p>
                                    Let’s build a single workspace for all of
                                    our coding agents. Keep it fast, focused,
                                    and close to the terminal.
                                  </p>
                                </div>
                                <div className="assistant-message">
                                  <div className="assistant-label">
                                    <AgentIcon agent={session.agent} />
                                    <strong>{session.agent}</strong>
                                    <span>12:39 PM</span>
                                  </div>
                                  <p>
                                    I’ll start with the companion interface:
                                    your sessions on the left, the conversation
                                    in the center, and a live review panel
                                    alongside it.
                                  </p>
                                  <ToolCard
                                    title="Inspect workspace"
                                    lines={[
                                      "ls crates/agmux-cli/src",
                                      "cat companion_gui.rs",
                                    ]}
                                  />
                                  <p>
                                    The Rust backend already has the building
                                    blocks we need: authenticated IPC, session
                                    viewports, and an event stream.
                                  </p>
                                  <p>
                                    I’ve put together the first pass of the
                                    workspace:
                                  </p>
                                  <ul className="result-list">
                                    <li>
                                      <Check size={13} />
                                      One inbox for every agent and session
                                    </li>
                                    <li>
                                      <Check size={13} />
                                      Chat, files, Git, and terminal in one
                                      place
                                    </li>
                                    <li>
                                      <Check size={13} />
                                      Changes ready to review alongside the
                                      conversation
                                    </li>
                                  </ul>
                                  <ToolCard
                                    title="Update interface"
                                    lines={"src/components/SessionList.tsx|src/components/ChatView.tsx|src/theme.ts".split(
                                      "|",
                                    )}
                                  />
                                  <div className="change-summary">
                                    <GitCompareArrows size={14} />
                                    <span>3 files changed</span>
                                    <b className="success">+144</b>
                                    <b className="danger">−23</b>
                                    <button
                                      onClick={() => {
                                        setReview(true);
                                        setTab("Git");
                                      }}
                                    >
                                      Review changes
                                      <ArrowRight size={13} />
                                    </button>
                                  </div>
                                  <p>
                                    The UI is ready for a first look. You can
                                    switch agents, open a terminal, or review
                                    the changes without leaving the session.
                                  </p>
                                </div>
                                <div className="timeline-complete">
                                  <span>
                                    <CircleCheck size={17} />
                                  </span>
                                  <div>
                                    <strong>Interface pass complete</strong>
                                    <small>
                                      3 files updated · ready for your review
                                    </small>
                                  </div>
                                  <button
                                    onClick={() => {
                                      setReview(true);
                                      setFocusView(false);
                                    }}
                                  >
                                    Review
                                    <ArrowRight size={13} />
                                  </button>
                                </div>
                                <AnimatePresence initial={false} mode="wait">
                                  {!approved[session.id] ? (
                                    <motion.div
                                      key="permission-request"
                                      className="approval-card"
                                      initial={{ opacity: 0, height: 0 }}
                                      animate={{ opacity: 1, height: "auto" }}
                                      exit={{
                                        opacity: 0,
                                        height: 0,
                                        marginTop: 0,
                                        marginBottom: 0,
                                        paddingTop: 0,
                                        paddingBottom: 0,
                                      }}
                                      transition={motionTiming(
                                        reduced,
                                        "disclosure",
                                      )}
                                      style={{ overflow: "hidden" }}
                                    >
                                      <div className="row">
                                        <ShieldCheck size={17} />
                                        <strong>Permission requested</strong>
                                        <span className="approval-demo">
                                          Demo
                                        </span>
                                      </div>
                                      <p>Run the workspace checks?</p>
                                      <code>cargo check --workspace</code>
                                      <div className="approval-actions">
                                        <span>Terminal · /workspace/agmux</span>
                                        <Action
                                          onPress={() => {
                                            setApproved((prev) => ({
                                              ...prev,
                                              [session.id]: "denied",
                                            }));
                                            notify("Demo permission declined");
                                          }}
                                        >
                                          Deny
                                        </Action>
                                        <Action
                                          primary
                                          onPress={() => {
                                            setApproved((prev) => ({
                                              ...prev,
                                              [session.id]: "allowed",
                                            }));
                                            notify(
                                              "Demo permission approved · no command executed",
                                            );
                                          }}
                                        >
                                          Allow once
                                        </Action>
                                      </div>
                                    </motion.div>
                                  ) : (
                                    <motion.div
                                      key="permission-resolved"
                                      className={
                                        "resolved-permission permission-" +
                                        approved[session.id]
                                      }
                                      initial={{
                                        opacity: 0,
                                        y: reduced ? 0 : 5,
                                      }}
                                      animate={{ opacity: 1, y: 0 }}
                                      transition={motionTiming(
                                        reduced,
                                        "completion",
                                      )}
                                    >
                                      {approved[session.id] === "allowed" ? (
                                        <AnimatedCheck size={21} />
                                      ) : (
                                        <X size={18} />
                                      )}
                                      <div>
                                        <strong>
                                          Permission request resolved in preview
                                        </strong>
                                        <small>
                                          {approved[session.id] === "allowed"
                                            ? "Allowed once · no command executed"
                                            : "Denied · no command executed"}
                                        </small>
                                      </div>
                                    </motion.div>
                                  )}
                                </AnimatePresence>
                              </>
                            ) : messages[session.id]?.length ? (
                              <div className="session-context">
                                <AgentIcon agent={session.agent} />
                                <span>{session.title}</span>
                                <small>{session.project}</small>
                              </div>
                            ) : (
                              <EmptyState
                                kind="conversation"
                                title="What are we building?"
                                description={`A fresh session with ${session.agent}. Give it a goal, an idea, or a problem worth solving.`}
                              >
                                <div className="starter-prompts">
                                  {[
                                    "Explore this codebase",
                                    "Plan a new feature",
                                    "Review recent changes",
                                  ].map((prompt) => (
                                    <button
                                      key={prompt}
                                      onClick={() =>
                                        setDrafts((prev) => ({
                                          ...prev,
                                          [session.id]: prompt,
                                        }))
                                      }
                                    >
                                      {prompt}
                                      <ArrowRight size={13} />
                                    </button>
                                  ))}
                                </div>
                              </EmptyState>
                            )}
                          </>
                        }
                      />
                    )}
                  </motion.div>
                  {sessions.length > 0 && (
                    <Composer
                      session={session}
                      sessions={sessions}
                      draft={draft}
                      onDraft={(text) =>
                        setDrafts((prev) => ({ ...prev, [session.id]: text }))
                      }
                      onSend={send}
                      running={!!running[session.id]}
                      onStop={() => stopTurn(session.id)}
                      mode={mode}
                      onMode={() => setMenu(menu === "mode" ? null : "mode")}
                      onAgent={() => {
                        setMenu(menu === "agent" ? null : "agent");
                        setAgentSearch("");
                      }}
                      agentIcon={<AgentIcon agent={session.agent} />}
                      queue={queues[session.id] || []}
                      onQueueMove={(id, delta) =>
                        updateQueue(session.id, (queue) =>
                          moveQueuedMessage(queue, id, delta),
                        )
                      }
                      onQueueDelete={(id) =>
                        updateQueue(session.id, (queue) =>
                          queue.filter((item) => item.id !== id),
                        )
                      }
                      onQueueEdit={(id) => {
                        const item = queuesRef.current[session.id]?.find(
                          (item) => item.id === id,
                        );
                        if (item) {
                          setDrafts((prev) => ({
                            ...prev,
                            [session.id]: [
                              prev[session.id],
                              item.text,
                              ...item.attachments.map((name) => `📎 ${name}`),
                            ]
                              .filter(Boolean)
                              .join("\n"),
                          }));
                          updateQueue(session.id, (queue) =>
                            queue.filter((item) => item.id !== id),
                          );
                        }
                      }}
                      onQueueResume={() => {
                        const next = queuesRef.current[session.id]?.[0];
                        if (
                          next &&
                          !running[session.id] &&
                          session.state !== "reconnecting"
                        ) {
                          updateQueue(session.id, (queue) => queue.slice(1));
                          startTurn(
                            session.id,
                            next.text,
                            next.attachments,
                            next.id,
                          );
                        }
                      }}
                      attachments={attachments[session.id] || []}
                      onAttach={addAttachments}
                      onRemoveAttachment={(id) => {
                        const item = attachments[session.id]?.find(
                          (item) => item.id === id,
                        );
                        if (item?.url) {
                          URL.revokeObjectURL(item.url);
                          objectURLs.current.delete(item.url);
                        }
                        setAttachments((prev) => ({
                          ...prev,
                          [session.id]: (prev[session.id] || []).filter(
                            (item) => item.id !== id,
                          ),
                        }));
                      }}
                      onLatest={() => transcript.current?.jumpToLatest()}
                      onSettings={() => setModal("settings")}
                    />
                  )}
                </>
              )}
              {tab === "Terminal" && (
                <div className="terminal-view">
                  <div className="terminal-top">
                    <span>
                      <SquareTerminal size={14} />
                      {session.agent}{" "}
                      <span className="muted">/ {session.project}</span>
                    </span>
                    <span className="tiny-badge">DEMO SHELL</span>
                  </div>
                  <Suspense
                    fallback={
                      <div className="empty-search">Opening terminal…</div>
                    }
                  >
                    <Terminal sessionId={session.id} light={light} />
                  </Suspense>
                  <div className="terminal-footer">
                    <ShieldCheck size={12} />
                    Local simulator · Rust PTY connection pending
                  </div>
                </div>
              )}
              {tab === "Files" && (
                <div className="files-view">
                  <div className="file-tree">
                    <span className="section-label">
                      EXPLORER <span className="tiny-badge">DEMO</span>
                    </span>
                    <div className="tree-folder">
                      <ChevronDown size={12} />
                      <Folder size={14} />
                      src
                    </div>
                    {files.map((item, index) => (
                      <button
                        key={item.path}
                        className={file === index ? "active" : ""}
                        onClick={() => {
                          setFile(index);
                          setEditor(
                            documents[item.path] ??
                              (index === 0
                                ? source
                                : `// ${item.path}\n\nexport const background = '#171615';\nexport const surface = '#211e1b';\n`),
                          );
                        }}
                      >
                        <FileCode2 size={13} />
                        {item.path.split("/").pop()}
                        <span>M</span>
                      </button>
                    ))}
                  </div>
                  <div className="editor-pane">
                    <div className="editor-tab">
                      <FileCode2 size={14} />
                      {files[file].path.split("/").pop()}
                      <span className="muted">Demo file</span>
                      <button
                        onClick={() => {
                          localStorage.setItem(
                            "agmux-demo-files",
                            JSON.stringify({
                              ...documents,
                              [files[file].path]: editor,
                            }),
                          );
                          notify("Saved to this browser");
                        }}
                      >
                        Save
                      </button>
                    </div>
                    <Suspense
                      fallback={
                        <div className="empty-search">Opening editor…</div>
                      }
                    >
                      <CodeEditor
                        value={editor}
                        file={files[file].path}
                        light={light}
                        onChange={(value) => {
                          setEditor(value);
                          setDocuments((prev) => ({
                            ...prev,
                            [files[file].path]: value,
                          }));
                        }}
                        onSave={() => {
                          localStorage.setItem(
                            "agmux-demo-files",
                            JSON.stringify({
                              ...documents,
                              [files[file].path]: editor,
                            }),
                          );
                          notify("Saved to this browser");
                        }}
                      />
                    </Suspense>
                  </div>
                </div>
              )}
              {tab === "Git" && (
                <div className="git-view">
                  <div className="git-intro">
                    <GitBranch size={19} />
                    <div>
                      <h2>Workspace changes</h2>
                      <p>
                        companion-ui <span>→ main</span>
                      </p>
                    </div>
                    <span className="tiny-badge">DEMO DIFF</span>
                  </div>
                  <div className="git-view-controls">
                    <div
                      className="segmented-control diff-layout-control"
                      aria-label="Diff layout"
                    >
                      {(["unified", "split"] as const).map((layout) => (
                        <button
                          key={layout}
                          aria-pressed={diffLayout === layout}
                          onClick={() => setDiffLayout(layout)}
                        >
                          <Selection
                            active={diffLayout === layout}
                            id="git-diff-selection"
                          />
                          {layout === "unified" ? "Unified" : "Side by side"}
                        </button>
                      ))}
                    </div>
                    <span className="diff-legend">
                      <i />
                      Added
                      <i />
                      Removed
                    </span>
                  </div>
                  <div className="git-summary">
                    <span>3 files changed</span>
                    <b className="success">+144</b>
                    <b className="danger">−23</b>
                    <Action
                      primary
                      onPress={() => {
                        setReviewed(!reviewed);
                        notify(
                          reviewed
                            ? "Review reopened"
                            : "Demo changes marked as reviewed",
                        );
                      }}
                    >
                      {reviewed ? "Reopen review" : "Mark reviewed"}
                    </Action>
                  </div>
                  {files.map((item, index) => (
                    <div key={item.path} className="git-file">
                      <button
                        className="diff-file-header"
                        onClick={() => setFile(index)}
                      >
                        <ChevronDown size={12} />
                        <FileCode2 size={13} />
                        <span>{item.path}</span>
                        <b className="success">+{item.add}</b>
                        <b className="danger">−{item.del}</b>
                      </button>
                      <Diff file={index} layout={diffLayout} />
                    </div>
                  ))}
                </div>
              )}
              {tab === "Machines" && (
                <div className="machines-view">
                  <div className="page-intro">
                    <span className="eyebrow">Your workspace, everywhere</span>
                    <h2>Machines</h2>
                    <p>The places your agents call home.</p>
                  </div>
                  {machineOffline ? (
                    <EmptyState
                      kind="disconnected"
                      title="This machine is taking a break"
                      description="Your sessions and drafts are safe. A live connection will resume when your machine is reachable again. This is an offline preview."
                      action="Back to machine preview"
                      onAction={() => setMachineOffline(false)}
                    />
                  ) : (
                    <div className="machine-preview-card">
                      <div className="machine-avatar">
                        <Monitor size={27} />
                      </div>
                      <div>
                        <h3>Local machine</h3>
                        <p>Agmux workspace · browser preview</p>
                        <span className="machine-status">
                          <i className="status-dot green" />
                          Preview available
                        </span>
                      </div>
                      <button onClick={() => setMachineOffline(true)}>
                        Preview offline state
                        <ArrowRight size={14} />
                      </button>
                    </div>
                  )}
                </div>
              )}
              <AnimatePresence initial={false}>
                {(menu === "agent" || menu === "mode") && (
                  <AnimatedPopover key={menu} className="agent-popover popover">
                    <span className="menu-label">
                      {menu === "agent"
                        ? "Choose an agent · demo"
                        : "Session mode · demo"}
                    </span>
                    {menu === "agent" && (
                      <input
                        autoFocus
                        aria-label="Search agents"
                        className="agent-search"
                        placeholder="Search agents…"
                        value={agentSearch}
                        onChange={(event) => setAgentSearch(event.target.value)}
                      />
                    )}
                    {(menu === "agent"
                      ? agents.filter((agent) =>
                          agent
                            .toLowerCase()
                            .includes(agentSearch.toLowerCase()),
                        )
                      : ["Default", "Plan", "Read-only"]
                    ).map((name) => (
                      <button
                        key={name}
                        onClick={() => {
                          if (menu === "agent")
                            setSessions((prev) =>
                              prev.map((item) =>
                                item.id === session.id
                                  ? { ...item, agent: name as Agent }
                                  : item,
                              ),
                            );
                          else setMode(name);
                          if (menu === "agent") {
                            if (menuCloseTimer.current)
                              clearTimeout(menuCloseTimer.current);
                            menuCloseTimer.current = setTimeout(
                              () => setMenu(null),
                              reduced ? 0 : 160,
                            );
                          } else setMenu(null);
                        }}
                      >
                        <Selection
                          active={
                            (menu === "agent" ? session.agent : mode) === name
                          }
                          id="agent-picker-selection"
                        />
                        {menu === "agent" && (
                          <AgentIcon agent={name as Agent} />
                        )}
                        {name}
                        {(menu === "agent" ? session.agent : mode) === name && (
                          <Check size={13} />
                        )}
                      </button>
                    ))}
                  </AnimatedPopover>
                )}
              </AnimatePresence>
            </section>
            <AnimatePresence initial={false}>
              {review && !focusView && sessions.length > 0 && (
                <motion.div
                  className="review-dock"
                  key="review-dock"
                  initial={{ width: 0, opacity: 0 }}
                  animate={{ width: reviewDockWidth, opacity: 1 }}
                  exit={{ width: 0, opacity: 0 }}
                  transition={
                    resizing ? { duration: 0 } : motionTiming(reduced, "panel")
                  }
                >
                  <PanelDivider
                    width={reviewWidth}
                    onResizeChange={setResizing}
                    onChange={(width) => {
                      setWideReview(false);
                      setReviewWidth(width);
                    }}
                  />
                  <aside
                    className={"review-pane" + (wideReview ? " wide" : "")}
                    style={{
                      width: reviewDockWidth - 7,
                      minWidth: reviewDockWidth - 7,
                    }}
                  >
                    <div className="review-top">
                      <span className="review-tab">
                        <GitCompareArrows size={13} />
                        Review<span>3</span>
                      </span>
                      <div className="row">
                        <IconButton
                          label="Expand review"
                          onClick={() => setWideReview(!wideReview)}
                        >
                          <Maximize2 size={13} />
                        </IconButton>
                        <IconButton
                          label="Close review"
                          onClick={() => setReview(false)}
                        >
                          <X size={15} />
                        </IconButton>
                      </div>
                    </div>
                    <div className="review-toolbar">
                      <div
                        className="segmented-control diff-layout-control"
                        aria-label="Diff layout"
                      >
                        {(["unified", "split"] as const).map((layout) => (
                          <button
                            key={layout}
                            aria-pressed={diffLayout === layout}
                            onClick={() => setDiffLayout(layout)}
                          >
                            <Selection
                              active={diffLayout === layout}
                              id="review-diff-selection"
                            />
                            {layout === "unified" ? "Unified" : "Side by side"}
                          </button>
                        ))}
                      </div>
                      <span className="tiny-badge">DEMO</span>
                    </div>
                    <div className="scope-tabs">
                      {["Included", "Pending", "Combined"].map((name) => (
                        <button
                          key={name}
                          className={reviewScope === name ? "active" : ""}
                          onClick={() => setReviewScope(name)}
                        >
                          <Selection
                            active={reviewScope === name}
                            id="review-scope-selection"
                          />
                          {name}
                        </button>
                      ))}
                    </div>
                    <div className="review-description">
                      {reviewScope === "Pending"
                        ? "No unstaged changes in this preview."
                        : "Changes from this session, ready for a closer look."}
                    </div>
                    {reviewed ? (
                      <EmptyState
                        compact
                        kind="completed"
                        title="Review complete"
                        description="Every change has had a closer look. You're ready for the next step."
                        action="Reopen review"
                        onAction={() => setReviewed(false)}
                      />
                    ) : reviewScope === "Pending" ? (
                      <EmptyState
                        compact
                        kind="completed"
                        title="All caught up"
                        description="No pending changes."
                      />
                    ) : (
                      <>
                        <div className="changes-heading">
                          <span>
                            Latest turn changes <span>(3)</span>
                          </span>
                          <span>
                            <b className="success">+144</b>{" "}
                            <b className="danger">−23</b>
                          </span>
                        </div>
                        <div className="changed-files">
                          {files.map((item, index) => (
                            <button
                              key={item.path}
                              className={file === index ? "active" : ""}
                              onClick={() => setFile(index)}
                            >
                              <span className="file-status">M</span>
                              <FileCode2 size={13} />
                              <span>{item.path.split("/").pop()}</span>
                              <b className="success">+{item.add}</b>
                              <b className="danger">−{item.del}</b>
                            </button>
                          ))}
                        </div>
                        <div className="diff-path">
                          <ChevronDown size={12} />
                          <span>{files[file].path}</span>
                          <IconButton
                            label="Open file"
                            onClick={() => {
                              setTab("Files");
                              setEditor(
                                file === 0
                                  ? source
                                  : `// ${files[file].path}\n`,
                              );
                            }}
                          >
                            <FileCode2 size={13} />
                          </IconButton>
                        </div>
                        <div className="review-scroll">
                          <Diff file={file} layout={diffLayout} />
                        </div>
                        <div className="review-bottom">
                          <span>
                            <CircleCheck size={13} />
                            {reviewed ? "Reviewed" : "Ready for review"}
                          </span>
                          <button
                            onClick={() => {
                              setReviewed(!reviewed);
                              notify(
                                reviewed
                                  ? "Review reopened"
                                  : "Demo review complete",
                              );
                            }}
                          >
                            {reviewed ? "Reopen" : "Mark reviewed"}
                            <Check size={12} />
                          </button>
                        </div>
                      </>
                    )}
                  </aside>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
          <footer className="status-bar">
            <span>
              <GitBranch size={11} />
              companion-ui <span className="muted">*</span>
            </span>
            <span>
              <i className="status-dot green" />
              {sessions.filter((item) => item.state === "working").length}{" "}
              agents working
            </span>
            <button
              onClick={() => {
                const attention = sessions.find(
                  (item) => item.state === "attention",
                );
                if (attention) choose(attention.id);
              }}
            >
              <Bell size={11} />
              {
                sessions.filter((item) => item.state === "attention").length
              }{" "}
              need attention
            </button>
            <span className="status-spacer" />
            <button onClick={() => setModal("commands")}>
              <Command size={11} />
              Command palette
            </button>
            <span className="muted">Agmux · UI preview</span>
          </footer>
        </main>
        <AnimatePresence initial={false}>
          {toast && (
            <motion.div
              key={toast + toastVersion}
              className="toast"
              role="status"
              style={{ x: "-50%" }}
              initial={{
                opacity: 0,
                y: reduced ? 0 : 12,
                scale: reduced ? 1 : 0.98,
              }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{
                opacity: 0,
                y: reduced ? 0 : -5,
                scale: reduced ? 1 : 0.98,
              }}
              transition={motionTiming(reduced, "overlay")}
            >
              <AnimatedCheck size={18} />
              {toast}
              <button
                aria-label="Dismiss notification"
                onClick={() => setToast("")}
              >
                <X size={13} />
              </button>
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence initial={false}>
          {modal && (
            <AnimatedOverlay
              onMouseDown={(e) => {
                if (e.target === e.currentTarget) setModal(null);
              }}
            >
              <motion.section
                initial={{
                  opacity: 0,
                  scale: reduced ? 1 : 0.98,
                  y: reduced ? 0 : 8,
                }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{
                  opacity: 0,
                  scale: reduced ? 1 : 0.98,
                  y: reduced ? 0 : 5,
                }}
                transition={motionTiming(reduced, "navigation")}
                className={
                  "modal " + (modal === "commands" ? "command-modal" : "")
                }
                role="dialog"
                aria-modal="true"
                aria-label={
                  modal === "connect" ? "Connect backend" : modal === "new"
                    ? "New session"
                    : modal === "settings"
                      ? "Settings"
                      : "Command palette"
                }
              >
                <div className="modal-heading">
                  <h2>
                    {modal === "connect" ? "Connect to your machine" : modal === "new"
                      ? "Start a new session"
                      : modal === "settings"
                        ? "Workspace settings"
                        : "Command palette"}
                  </h2>
                  <IconButton
                    label="Close dialog"
                    onClick={() => setModal(null)}
                  >
                    <X size={17} />
                  </IconButton>
                </div>
                {modal === "connect" && <ConnectBackend />}
                {modal === "new" && (
                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      create();
                    }}
                  >
                    <p className="modal-description">
                      A fresh conversation, on your machine, with your agent.
                    </p>
                    <label>
                      Session name
                      <input
                        autoFocus
                        required
                        placeholder="What are you working on?"
                        value={newTitle}
                        onChange={(e) => setNewTitle(e.target.value)}
                      />
                    </label>
                    <label>
                      Project directory
                      <input
                        required
                        value={newProject}
                        onChange={(e) => setNewProject(e.target.value)}
                      />
                    </label>
                    <label>Agent</label>
                    <div className="agent-grid">
                      {agents.map((agent) => (
                        <button
                          type="button"
                          className={newAgent === agent ? "selected" : ""}
                          key={agent}
                          onClick={() => setNewAgent(agent)}
                        >
                          <Selection
                            active={newAgent === agent}
                            id="new-session-agent-selection"
                          />
                          <AgentIcon agent={agent} />
                          {agent}
                          {newAgent === agent && <Check size={13} />}
                        </button>
                      ))}
                    </div>
                    <div className="machine-card">
                      <Monitor size={18} />
                      <div>
                        <strong>Local machine</strong>
                        <span>This browser · demo session</span>
                      </div>
                      <i className="status-dot green" />
                    </div>
                    <p className="demo-note">
                      UI preview. Creating a session won’t launch an agent.
                    </p>
                    <div className="modal-footer">
                      <Action onPress={() => setModal(null)}>Cancel</Action>
                      <button className="primary-button" type="submit">
                        Create session
                        <ArrowRight size={14} />
                      </button>
                    </div>
                  </form>
                )}
                {modal === "settings" && (
                  <>
                    <p className="modal-description">
                      Make this workspace feel like yours.
                    </p>
                    <div className="setting-row">
                      <div>
                        <strong>Appearance</strong>
                        <span>Warm, quiet colors. Built for focus.</span>
                      </div>
                      <div
                        className="segmented-control"
                        aria-label="Appearance"
                      >
                        <button
                          aria-pressed={!light}
                          onClick={() => setLight(false)}
                        >
                          <Selection
                            active={!light}
                            id="appearance-selection"
                          />
                          Dark
                        </button>
                        <button
                          aria-pressed={light}
                          onClick={() => setLight(true)}
                        >
                          <Selection active={light} id="appearance-selection" />
                          Light
                        </button>
                      </div>
                    </div>
                    <div className="setting-row">
                      <div>
                        <strong>Display density</strong>
                        <span>More breathing room, or more in view.</span>
                      </div>
                      <div
                        className="segmented-control"
                        aria-label="Display density"
                      >
                        {(["comfortable", "compact"] as const).map((value) => (
                          <button
                            key={value}
                            aria-pressed={density === value}
                            onClick={() => setDensity(value)}
                          >
                            <Selection
                              active={density === value}
                              id="density-selection"
                            />
                            {value === "comfortable"
                              ? "Comfortable"
                              : "Compact"}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div className="setting-row">
                      <div>
                        <strong>Default agent</strong>
                        <span>Used for new preview sessions.</span>
                      </div>
                      <select
                        aria-label="Default agent"
                        value={newAgent}
                        onChange={(e) => setNewAgent(e.target.value as Agent)}
                      >
                        {agents.map((agent) => (
                          <option key={agent}>{agent}</option>
                        ))}
                      </select>
                    </div>
                    <div className="settings-section">
                      <Monitor size={15} />
                      Local machine
                      <span className="connection-label">Preview only</span>
                    </div>
                    <p className="modal-description">
                      This mockup runs locally. Provider accounts and the Rust
                      daemon are not connected. Your demo conversations are
                      saved in this browser.
                    </p>
                    <div className="settings-section">
                      <Command size={15} />
                      Keyboard shortcuts
                    </div>
                    <div className="shortcut-row">
                      <span>Command palette</span>
                      <kbd>⌘ / Ctrl K</kbd>
                    </div>
                    <div className="shortcut-row">
                      <span>New session</span>
                      <kbd>⌘ / Ctrl N</kbd>
                    </div>
                    <div className="shortcut-row">
                      <span>Switch session</span>
                      <kbd>Alt ↑ / ↓</kbd>
                    </div>
                    <div className="shortcut-row">
                      <span>Quick agent picker</span>
                      <kbd>Alt A</kbd>
                    </div>
                    <div className="shortcut-row">
                      <span>Focus view</span>
                      <kbd>Alt F · Esc to exit</kbd>
                    </div>
                    <div className="shortcut-row">
                      <span>Send message</span>
                      <kbd>Enter</kbd>
                    </div>
                    <div className="modal-footer">
                      <Action primary onPress={() => setModal(null)}>
                        Done
                      </Action>
                    </div>
                  </>
                )}
                {modal === "commands" && (
                  <>
                    <div className="command-search">
                      <Search size={18} />
                      <input
                        autoFocus
                        aria-label="Search commands"
                        placeholder="Search commands or jump to a session…"
                        value={command}
                        onChange={(e) => {
                          setCommand(e.target.value);
                          setCommandIndex(0);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter")
                            commandItems[
                              Math.min(commandIndex, commandItems.length - 1)
                            ]?.action();
                          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                            e.preventDefault();
                            setCommandIndex(
                              (value) =>
                                (value +
                                  (e.key === "ArrowDown" ? 1 : -1) +
                                  commandItems.length) %
                                Math.max(1, commandItems.length),
                            );
                          }
                        }}
                      />
                      <kbd>esc</kbd>
                    </div>
                    <div className="command-results">
                      {commandItems.map((item, index) => (
                        <button
                          key={item.name + index}
                          className={commandIndex === index ? "selected" : ""}
                          onClick={item.action}
                        >
                          {item.icon}
                          <span>{item.name}</span>
                          <kbd>{item.key}</kbd>
                        </button>
                      ))}
                      {!commandItems.length && (
                        <p className="empty-search">
                          No matching commands or sessions.
                        </p>
                      )}
                    </div>
                    <div className="command-footer">
                      Your workspace, one shortcut away.<span>↵ to select</span>
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
