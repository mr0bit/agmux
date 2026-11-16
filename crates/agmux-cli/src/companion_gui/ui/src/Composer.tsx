import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { AnimatedPopover } from "./MotionUI";
import { motionTiming } from "./motion-spec";
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  ArrowDown,
  ArrowUp,
  ChevronDown,
  FileText,
  ListFilter,
  Paperclip,
  Pencil,
  Play,
  Plus,
  ShieldCheck,
  X,
} from "lucide-react";
import type { Agent, Session } from "./demo";
import { statuses, type QueuedMessage } from "./workspace";

export type Attachment = {
  id: string;
  name: string;
  size: number;
  type: string;
  url?: string;
  excerpt?: string;
};
export const agents: Agent[] = [
  "Claude Code",
  "Codex",
  "OpenCode",
  "Gemini",
  "Cursor",
  "Custom CLI",
];
export default function Composer({
  session,
  sessions,
  draft,
  onDraft,
  onSend,
  running,
  onStop,
  mode,
  onMode,
  onAgent,
  agentIcon,
  queue,
  onQueueMove,
  onQueueDelete,
  onQueueEdit,
  onQueueResume,
  attachments,
  onAttach,
  onRemoveAttachment,
  onLatest,
  onSettings,
  live = false,
  disabled: externallyDisabled = false,
}: {
  session: Session;
  sessions: Session[];
  draft: string;
  onDraft: (text: string) => void;
  onSend: () => void;
  running: boolean;
  onStop: () => void;
  mode: string;
  onMode: () => void;
  onAgent: () => void;
  agentIcon: ReactNode;
  queue: QueuedMessage[];
  onQueueMove: (id: string, delta: number) => void;
  onQueueDelete: (id: string) => void;
  onQueueEdit: (id: string) => void;
  onQueueResume: () => void;
  attachments: Attachment[];
  onAttach: (files: File[]) => void;
  onRemoveAttachment: (id: string) => void;
  onLatest: () => void;
  onSettings: () => void;
  live?: boolean;
  disabled?: boolean;
}) {
  const reduced = !!useReducedMotion();
  const input = useRef<HTMLTextAreaElement>(null);
  const upload = useRef<HTMLInputElement>(null);
  const [mention, setMention] = useState<{
    query: string;
    start: number;
    end: number;
  } | null>(null);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [dragging, setDragging] = useState(false);
  const disabled = externallyDisabled || session.state === "reconnecting";
  const state = statuses[session.state];
  useLayoutEffect(() => {
    const element = input.current;
    if (!element) return;
    element.style.height = "0px";
    element.style.height = `${Math.max(64, Math.min(180, element.scrollHeight))}px`;
  }, [draft, session.id]);
  useEffect(() => {
    setMention(null);
  }, [session.id]);
  const options = [
    ...agents.map((name) => ({
      id: name,
      label: name,
      detail: "Agent",
      value: name,
    })),
    ...sessions
      .filter((item) => item.id !== session.id)
      .map((item) => ({
        id: item.id,
        label: item.title,
        detail: item.agent,
        value: `{${item.title}}`,
      })),
  ]
    .filter((item) =>
      item.label.toLowerCase().includes(mention?.query.toLowerCase() || ""),
    )
    .sort((a, b) => {
      const query = mention?.query.toLowerCase() || "";
      return Number(b.label.toLowerCase().startsWith(query)) - Number(a.label.toLowerCase().startsWith(query));
    })
    .slice(0, 7);
  const trackMention = (text: string, cursor: number) => {
    if (live) {
      setMention(null);
      return;
    }
    const match = text.slice(0, cursor).match(/(?:^|\s)@([^\s@{}]*)$/);
    setMention(
      match
        ? { query: match[1], start: cursor - match[1].length - 1, end: cursor }
        : null,
    );
    setMentionIndex(0);
  };
  const selectMention = (index: number) => {
    const option = options[index];
    if (!mention || !option) return;
    const text =
      draft.slice(0, mention.start) +
      "@" +
      option.value +
      " " +
      draft.slice(mention.end);
    const cursor = mention.start + option.value.length + 2;
    onDraft(text);
    setMention(null);
    requestAnimationFrame(() => {
      input.current?.focus();
      input.current?.setSelectionRange(cursor, cursor);
    });
  };
  return (
    <div className="composer-area">
      <AnimatePresence initial={false}>
        {queue.length > 0 && (
          <motion.div
            key="queue"
            layoutScroll
            className="message-queue"
            aria-label="Queued messages"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{
              height: 0,
              opacity: 0,
              marginTop: 0,
              paddingTop: 0,
              paddingBottom: 0,
            }}
            transition={motionTiming(reduced, "disclosure")}
          >
            <div className="queue-heading">
              <span>
                {queue.length} queued ·{" "}
                {running ? "sends after this response" : "paused"}
              </span>
              {!running && (
                <button disabled={disabled} onClick={onQueueResume}>
                  <Play size={12} />
                  Resume queue
                </button>
              )}
            </div>
            <AnimatePresence initial={false}>
              {queue.map((item, index) => (
                <motion.div
                  className="queued-message"
                  key={item.id}
                  layout="position"
                  layoutId={`queued-${item.id}`}
                  initial={{ opacity: 0, y: reduced ? 0 : 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{
                    opacity: 0,
                    height: 0,
                    paddingTop: 0,
                    paddingBottom: 0,
                  }}
                  transition={motionTiming(reduced, "navigation")}
                >
                  <span className="queue-number">{index + 1}</span>
                  <span className="queue-text" title={item.text}>
                    {item.text}
                    {item.attachments.length > 0 && (
                      <small>
                        {" "}
                        · {item.attachments.length} attachment
                        {item.attachments.length === 1 ? "" : "s"}
                      </small>
                    )}
                  </span>
                  <button
                    aria-label={`Move queued message ${index + 1} up`}
                    disabled={index === 0}
                    onClick={() => onQueueMove(item.id, -1)}
                  >
                    <ArrowUp size={12} />
                  </button>
                  <button
                    aria-label={`Edit queued message ${index + 1}`}
                    onClick={() => onQueueEdit(item.id)}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    aria-label={`Remove queued message ${index + 1}`}
                    onClick={() => onQueueDelete(item.id)}
                  >
                    <X size={12} />
                  </button>
                </motion.div>
              ))}
            </AnimatePresence>
          </motion.div>
        )}
      </AnimatePresence>
      <div className="composer-status">
        <span className={"composer-session-state " + session.state}>
          <i className={"status-dot " + state.color} />
          {state.label}
          {disabled && " · input paused"}
        </span>
        {running && (
          <button className="stop-response" onClick={onStop}>
            Stop response
          </button>
        )}
        <button onClick={onLatest}>
          <ArrowDown size={12} />
          Latest message
        </button>
      </div>
      <div
        className={"composer" + (dragging ? " dragging" : "")}
        onDragOver={(event) => {
          if (!live && event.dataTransfer.types.includes("Files")) {
            event.preventDefault();
            setDragging(true);
          }
        }}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node))
            setDragging(false);
        }}
        onDrop={(event) => {
          event.preventDefault();
          setDragging(false);
          if (!disabled && !live)
            onAttach(Array.from(event.dataTransfer.files));
        }}
      >
        {attachments.length > 0 && (
          <div className="attachment-previews" aria-label="Attachment previews">
            {attachments.map((item) => (
              <div className="attachment-preview" key={item.id}>
                {item.url ? (
                  <img src={item.url} alt={`Preview of ${item.name}`} />
                ) : (
                  <span className="file-thumbnail">
                    <FileText size={20} />
                  </span>
                )}
                <div>
                  <strong>{item.name}</strong>
                  <span>
                    {item.size < 1024
                      ? `${item.size} B`
                      : `${Math.ceil(item.size / 1024)} KB`}
                    {item.excerpt && (
                      <span className="attachment-excerpt">{item.excerpt}</span>
                    )}
                  </span>
                </div>
                <button
                  aria-label={`Remove ${item.name}`}
                  onClick={() => onRemoveAttachment(item.id)}
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        )}
        <textarea
          ref={input}
          aria-label="Message agent"
          placeholder={
            disabled
              ? "Reconnecting… your draft is safe."
              : running
                ? "Queue the next instruction…"
                : "What would you like to work on?"
          }
          rows={2}
          value={draft}
          disabled={disabled}
          onChange={(event) => {
            onDraft(event.target.value);
            trackMention(event.target.value, event.target.selectionStart);
          }}
          onClick={(event) =>
            trackMention(
              event.currentTarget.value,
              event.currentTarget.selectionStart,
            )
          }
          onKeyDown={(event) => {
            if (event.nativeEvent.isComposing) return;
            if (mention && options.length) {
              if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                setMentionIndex(
                  (value) =>
                    (value +
                      (event.key === "ArrowDown" ? 1 : -1) +
                      options.length) %
                    options.length,
                );
                return;
              }
              if (event.key === "Enter" || event.key === "Tab") {
                event.preventDefault();
                selectMention(mentionIndex);
                return;
              }
            }
            if (event.key === "Escape") {
              setMention(null);
              return;
            }
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              onSend();
            }
          }}
          aria-expanded={!!mention}
          aria-activedescendant={
            mention && options.length
              ? `mention-option-${mentionIndex}`
              : undefined
          }
          aria-controls={mention ? "mention-options" : undefined}
        />
        <AnimatePresence initial={false}>
          {" "}
          {mention && (
            <AnimatedPopover
              className="mention-popover popover"
              id="mention-options"
              role="listbox"
              aria-label="Mention an agent or session"
            >
              <span className="menu-label">Mention an agent or session</span>
              {options.map((option, index) => (
                <button
                  key={option.id}
                  id={`mention-option-${index}`}
                  role="option"
                  aria-selected={index === mentionIndex}
                  className={index === mentionIndex ? "selected" : ""}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => selectMention(index)}
                >
                  <span>@</span>
                  <span>{option.label}</span>
                  <small>{option.detail}</small>
                </button>
              ))}
              {!options.length && (
                <p className="empty-search">No matches. Try an agent name.</p>
              )}
              <div className="mention-hint">
                ↑ ↓ to navigate · Enter to insert
              </div>
            </AnimatedPopover>
          )}
        </AnimatePresence>{" "}
        {dragging && (
          <div className="drop-hint">
            <Plus size={18} />
            Drop files to preview locally
          </div>
        )}
        <div className="composer-toolbar">
          <div className="row composer-options">
            <button onClick={onAgent} aria-label="Quick agent picker">
              {agentIcon}
              {session.agent}
              <ChevronDown size={11} />
            </button>
            <button onClick={onMode}>
              <ListFilter size={14} />
              {mode}
              <ChevronDown size={11} />
            </button>
            <button
              className="icon-button"
              aria-label="Permission settings"
              onClick={onSettings}
            >
              <ShieldCheck size={15} />
            </button>
          </div>
          <div className="row">
            <button
              className="icon-button"
              aria-label="Attach files"
              title="Attach files or drag them here"
              disabled={disabled || live}
              onClick={() => upload.current?.click()}
            >
              <Paperclip size={15} />
            </button>
            <button
              className="icon-button"
              aria-label="Mention an agent"
              title={
                live
                  ? "Use Send to session for agent-to-agent messages"
                  : "Mention an agent or session"
              }
              disabled={disabled || live}
              onClick={() => {
                const suffix = (draft && !/\s$/.test(draft) ? " " : "") + "@";
                const text = draft + suffix;
                onDraft(text);
                trackMention(text, text.length);
                input.current?.focus();
              }}
            >
              <span className="at-sign">@</span>
            </button>
            <motion.button
              whileTap={{ scale: reduced ? 1 : 0.94 }}
              className={"send-button" + (running ? " queue-send" : "")}
              disabled={!draft.trim() || disabled}
              aria-label={running ? "Queue message" : "Send message"}
              title={running ? "Queue message" : "Send message"}
              onClick={onSend}
            >
              {running ? <Plus size={17} /> : <ArrowUp size={17} />}
            </motion.button>
          </div>
        </div>
      </div>
      <div className="composer-footnote">
        {live
          ? "Live input · sent to the selected process"
          : "Preview · Nothing is sent to a provider"}
        <span>
          {running ? "Enter to queue" : "Enter to send"} · Shift ↵ for newline
        </span>
      </div>
      <input
        hidden
        type="file"
        multiple
        ref={upload}
        aria-label="Upload attachments"
        onChange={(event) => {
          onAttach(Array.from(event.target.files || []));
          event.target.value = "";
        }}
      />
    </div>
  );
}
