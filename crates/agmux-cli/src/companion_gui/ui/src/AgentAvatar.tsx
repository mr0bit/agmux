import {
  Bot,
  Code2,
  MousePointer2,
  Sparkles,
  Sun,
  TerminalSquare,
} from "lucide-react";
import type { Agent } from "./demo";
const identities = {
  "Claude Code": { name: "claude", icon: Sun },
  Codex: { name: "codex", icon: Bot },
  OpenCode: { name: "opencode", icon: Code2 },
  Gemini: { name: "gemini", icon: Sparkles },
  Cursor: { name: "cursor", icon: MousePointer2 },
  "Custom CLI": { name: "custom", icon: TerminalSquare },
} as const;
export default function AgentAvatar({ agent }: { agent: Agent }) {
  const identity =
    identities[agent as keyof typeof identities] || identities["Custom CLI"];
  const Icon = identity.icon;
  return (
    <span
      className={`agent-icon agent-${identity.name}`}
      title={agent}
      aria-hidden="true"
    >
      <Icon size={15} />
    </span>
  );
}
