import {
  ArrowRight,
  CheckCheck,
  FolderOpen,
  MessageSquare,
  MonitorOff,
  Search,
} from "lucide-react";
import { AnimatedCheck } from "./MotionUI";
import type { ReactNode } from "react";
const icons = {
  workspace: FolderOpen,
  conversation: MessageSquare,
  search: Search,
  disconnected: MonitorOff,
  completed: CheckCheck,
};
export default function EmptyState({
  kind,
  title,
  description,
  action,
  onAction,
  children,
  compact = false,
}: {
  kind: keyof typeof icons;
  title: string;
  description: string;
  action?: string;
  onAction?: () => void;
  children?: ReactNode;
  compact?: boolean;
}) {
  const Icon = icons[kind];
  return (
    <section
      className={`empty-state empty-${kind}${compact ? " compact-empty" : ""}`}
      aria-label={title}
    >
      <div className="empty-illustration" aria-hidden="true">
        <span className="empty-orbit" />
        <span className="empty-orbit second" />
        <span className="empty-symbol">
          {kind === "completed" ? (
            <AnimatedCheck size={compact ? 25 : 32} />
          ) : (
            <Icon size={compact ? 22 : 29} />
          )}
        </span>
        <i />
        <i />
      </div>
      <span className="empty-eyebrow">
        {kind === "completed"
          ? "A little less to think about"
          : kind === "disconnected"
            ? "Pick up where you left off"
            : kind === "workspace"
              ? "Room for your next idea"
              : kind === "search"
                ? "A fresh search might help"
                : "A fresh conversation"}
      </span>
      <h2>{title}</h2>
      <p>{description}</p>
      {action && (
        <button className="empty-action" onClick={onAction}>
          {action}
          <ArrowRight size={14} />
        </button>
      )}
      {children}
    </section>
  );
}
