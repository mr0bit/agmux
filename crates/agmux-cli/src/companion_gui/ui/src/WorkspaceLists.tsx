import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { motionTiming } from "./motion-spec";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, ChevronDown, ListFilter } from "lucide-react";
import type { Agent, Message, Session } from "./demo";
import { clampReviewWidth, statuses } from "./workspace";

import AgentIcon from "./AgentAvatar";
import EmptyState from "./EmptyState";

type Group = { name: string; items: Session[] };
/** One virtualizer across all groups keeps the mounted session rows bounded. */
export function SessionList({
  groups,
  density,
  onClear,
  collapsed,
  onCollapse,
  selected,
  onSelect,
  onFilter,
  icon,
}: {
  groups: Group[];
  density: "comfortable" | "compact";
  onClear: () => void;
  collapsed: Record<string, boolean>;
  onCollapse: (name: string) => void;
  selected: string;
  onSelect: (id: string) => void;
  onFilter: () => void;
  icon: (session: Session) => ReactNode;
}) {
  const host = useRef<HTMLDivElement>(null);
  const rows = groups.flatMap((group) =>
    !group.items.length
      ? []
      : [
          { key: "heading-" + group.name, group, item: null as Session | null },
          ...(!collapsed[group.name]
            ? group.items.map((item) => ({ key: item.id, group, item }))
            : []),
        ],
  );
  const virtual = useVirtualizer({
    count: rows.length,
    getScrollElement: () => host.current,
    estimateSize: (index) =>
      rows[index].item
        ? density === "compact"
          ? 48
          : 66
        : density === "compact"
          ? 32
          : 40,
    getItemKey: (index) => rows[index].key,
    overscan: 5,
  });
  useEffect(() => {
    virtual.measure();
  }, [density]);
  useEffect(() => {
    const index = rows.findIndex((row) => row.item?.id === selected);
    if (index >= 0) virtual.scrollToIndex(index, { align: "auto" });
  }, [selected, collapsed]);
  return (
    <div
      className="session-groups virtual-sessions"
      ref={host}
      aria-label="Sessions"
      tabIndex={-1}
    >
      {!rows.length && (
        <EmptyState
          compact
          kind="search"
          title="Nothing here just yet"
          description="No sessions match your search or agent filter."
          action="Clear filters"
          onAction={onClear}
        />
      )}
      <div
        style={{
          height: virtual.getTotalSize(),
          position: "relative",
          width: "100%",
        }}
      >
        {virtual.getVirtualItems().map((row) => {
          const entry = rows[row.index];
          const item = entry.item;
          return (
            <div
              key={row.key}
              data-index={row.index}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: row.size,
                transform: `translateY(${row.start}px)`,
              }}
            >
              {!item ? (
                <div className="group-heading">
                  <button
                    aria-expanded={!collapsed[entry.group.name]}
                    onClick={() => onCollapse(entry.group.name)}
                  >
                    <ChevronDown
                      size={12}
                      className={collapsed[entry.group.name] ? "rotated" : ""}
                    />
                    {entry.group.name}
                    <span>{entry.group.items.length}</span>
                  </button>
                  <button
                    className="icon-button"
                    aria-label={`Filter ${entry.group.name} sessions`}
                    onClick={onFilter}
                  >
                    <ListFilter size={13} />
                  </button>
                </div>
              ) : (
                <button
                  className={
                    "session-item" + (item.id === selected ? " selected" : "")
                  }
                  onClick={() => onSelect(item.id)}
                  aria-current={item.id === selected ? "true" : undefined}
                  title={`${item.title} · ${statuses[item.state].label}`}
                >
                  {icon(item)}
                  <span className="session-label">
                    {item.title}
                    <span>
                      {item.project} · {item.agent}
                    </span>
                  </span>
                  <span className="session-meta">
                    <span className={"session-state " + item.state}>
                      <i
                        className={"status-dot " + statuses[item.state].color}
                      />
                      {statuses[item.state].label}
                    </span>
                    <time>{item.time}</time>
                  </span>
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export type TranscriptHandle = { jumpToLatest: () => void };
/** Dynamically measured transcript rows; only follow output when already at the bottom. */
export const Transcript = forwardRef<
  TranscriptHandle,
  {
    sessionId: string;
    intro: ReactNode;
    messages: Message[];
    agent: Agent;
    density: "comfortable" | "compact";
    running: boolean;
    onStop: () => void;
    positions: RefObject<Record<string, number>>;
  }
>(function Transcript(
  { sessionId, intro, messages, agent, density, running, onStop, positions },
  ref,
) {
  const reduced = !!useReducedMotion();
  const seenMessages = useRef(new Set(messages.map((message) => message.id)));
  useLayoutEffect(() => {
    messages.forEach((message) => seenMessages.current.add(message.id));
  }, [messages.length]);
  const host = useRef<HTMLDivElement>(null);
  const following = useRef(false);
  const mounted = useRef(false);
  const lastCount = useRef(messages.length);
  const [unread, setUnread] = useState(0);
  const [away, setAway] = useState(false);
  const count = 1 + messages.length + (running ? 1 : 0);
  const virtual = useVirtualizer({
    count,
    getScrollElement: () => host.current,
    estimateSize: (index) =>
      index === 0
        ? sessionId === "workspace"
          ? 1000
          : 250
        : index > messages.length
          ? 45
          : 160,
    getItemKey: (index) =>
      index === 0
        ? "intro"
        : index > messages.length
          ? "working"
          : messages[index - 1].id,
    overscan: 3,
    initialOffset: positions.current[sessionId] || 0,
  });
  useEffect(() => {
    virtual.measure();
  }, [density]);
  const jumpToLatest = () => {
    following.current = true;
    setAway(false);
    setUnread(0);
    virtual.scrollToIndex(count - 1, { align: "end" });
  };
  useImperativeHandle(ref, () => ({ jumpToLatest }));
  useLayoutEffect(() => {
    if (!mounted.current) {
      mounted.current = true;
      if (positions.current[sessionId] === undefined && messages.length > 0) {
        following.current = true;
        virtual.scrollToIndex(count - 1, { align: "end" });
      }
      return;
    }
    if (following.current) virtual.scrollToIndex(count - 1, { align: "end" });
    else if (messages.length > lastCount.current) {
      const delta = messages.length - lastCount.current;
      setUnread((value) => value + delta);
    }
    lastCount.current = messages.length;
  }, [messages, running]);
  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      if (following.current) virtual.scrollToIndex(count - 1, { align: "end" });
    });
    const content = element.firstElementChild;
    if (content) observer.observe(content);
    return () => observer.disconnect();
  }, [count]);
  return (
    <div className="transcript-shell">
      <motion.div
        layoutScroll
        ref={host}
        className="transcript virtual-transcript"
        aria-label="Conversation"
        onScroll={() => {
          const element = host.current!;
          positions.current[sessionId] = element.scrollTop;
          const atBottom =
            element.scrollHeight - element.scrollTop - element.clientHeight <
            70;
          following.current = atBottom;
          setAway(!atBottom);
          if (atBottom) setUnread(0);
        }}
      >
        <div
          style={{
            height: virtual.getTotalSize(),
            width: "100%",
            position: "relative",
          }}
        >
          {virtual.getVirtualItems().map((row) => (
            <div
              key={row.key}
              data-index={row.index}
              ref={virtual.measureElement}
              className="transcript-row"
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${row.start}px)`,
              }}
            >
              {row.index === 0 ? (
                intro
              ) : row.index > messages.length ? (
                <div className="thinking" role="status">
                  <span className="working-dots" aria-hidden="true">
                    <i />
                    <i />
                    <i />
                  </span>
                  Working in preview…<button onClick={onStop}>Stop</button>
                </div>
              ) : (
                <motion.div
                  layoutId={
                    messages[row.index - 1].role === "user"
                      ? `queued-${messages[row.index - 1].id}`
                      : undefined
                  }
                  layoutDependency={messages.length}
                  initial={
                    seenMessages.current.has(messages[row.index - 1].id)
                      ? false
                      : { opacity: 0, y: reduced ? 0 : 7 }
                  }
                  animate={{ opacity: 1, y: 0 }}
                  transition={motionTiming(reduced, "navigation")}
                  className={
                    messages[row.index - 1].role === "user"
                      ? "user-message"
                      : "assistant-message"
                  }
                >
                  <div className="message-author">
                    {messages[row.index - 1].role !== "user" && (
                      <AgentIcon agent={agent} />
                    )}
                    {messages[row.index - 1].role === "user" ? "You" : agent}
                    <span>just now</span>
                  </div>
                  <p className="message-text">
                    {messages[row.index - 1].text || "…"}
                  </p>
                </motion.div>
              )}
            </div>
          ))}
        </div>
      </motion.div>
      <AnimatePresence initial={false}>
        {" "}
        {away && messages.length > 0 && (
          <motion.button
            className="jump-latest"
            onClick={jumpToLatest}
            style={{ x: "-50%" }}
            initial={{ opacity: 0, y: reduced ? 0 : 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: reduced ? 0 : 4 }}
            transition={motionTiming(reduced, "overlay")}
          >
            <ArrowDown size={13} />
            {unread
              ? `${unread} new message${unread === 1 ? "" : "s"}`
              : "Jump to latest"}
          </motion.button>
        )}
      </AnimatePresence>{" "}
    </div>
  );
});

export function PanelDivider({
  width,
  onChange,
  onResizeChange,
}: {
  width: number;
  onChange: (width: number) => void;
  onResizeChange?: (resizing: boolean) => void;
}) {
  return (
    <div
      className="panel-divider"
      role="separator"
      aria-label="Resize review panel"
      aria-orientation="vertical"
      aria-valuemin={280}
      aria-valuemax={900}
      aria-valuenow={width}
      tabIndex={0}
      onDoubleClick={() => onChange(420)}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
          event.preventDefault();
          const available =
            event.currentTarget.closest<HTMLElement>(
              ".workspace-body",
            )!.clientWidth;
          onChange(
            clampReviewWidth(
              width + (event.key === "ArrowLeft" ? 24 : -24),
              available,
            ),
          );
        }
        if (event.key === "Home") onChange(420);
      }}
      onPointerDown={(event) => {
        const element = event.currentTarget;
        onResizeChange?.(true);
        const start = event.clientX;
        const initial = width;
        const available =
          element.closest<HTMLElement>(".workspace-body")!.clientWidth;
        element.setPointerCapture(event.pointerId);
        document.body.classList.add("resizing-panels");
        const move = (e: PointerEvent) =>
          onChange(clampReviewWidth(initial + start - e.clientX, available));
        const finish = () => {
          onResizeChange?.(false);
          element.removeEventListener("pointermove", move);
          element.removeEventListener("pointerup", finish);
          element.removeEventListener("pointercancel", finish);
          document.body.classList.remove("resizing-panels");
        };
        element.addEventListener("pointermove", move);
        element.addEventListener("pointerup", finish);
        element.addEventListener("pointercancel", finish);
      }}
    >
      <span />
    </div>
  );
}
