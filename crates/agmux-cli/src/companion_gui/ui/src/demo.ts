export type Agent =
  | "Claude Code"
  | "Codex"
  | "OpenCode"
  | "Gemini"
  | "Cursor"
  | "Custom CLI"
  | (string & {});
export type Session = {
  id: string;
  title: string;
  agent: Agent;
  state: "attention" | "working" | "idle" | "reconnecting" | "stopped";
  time: string;
  project: string;
  pinned?: boolean;
};
export type Message = { id: string; role: "user" | "assistant"; text: string };
export const initialSessions: Session[] = [
  {
    id: "workspace",
    title: "Build the companion workspace",
    agent: "Claude Code",
    state: "attention",
    time: "now",
    project: "agmux",
  },
  {
    id: "pty",
    title: "Review PTY lifecycle",
    agent: "Codex",
    state: "attention",
    time: "4m",
    project: "agmux",
  },
  {
    id: "stream",
    title: "Optimize session streaming",
    agent: "Codex",
    state: "working",
    time: "2m",
    project: "agmux",
  },
  {
    id: "terminal",
    title: "Terminal rendering improvements",
    agent: "OpenCode",
    state: "working",
    time: "8m",
    project: "agmux",
  },
  {
    id: "plugins",
    title: "Agent adapter discovery",
    agent: "Gemini",
    state: "working",
    time: "12m",
    project: "agmux",
  },
  {
    id: "ipc",
    title: "IPC transport audit",
    agent: "Codex",
    state: "idle",
    time: "1h",
    project: "agmux",
    pinned: true,
  },
  {
    id: "worktrees",
    title: "Git worktree isolation",
    agent: "Claude Code",
    state: "idle",
    time: "2h",
    project: "agmux",
    pinned: true,
  },
  {
    id: "notes",
    title: "Companion architecture notes",
    agent: "OpenCode",
    state: "idle",
    time: "3h",
    project: "agmux",
    pinned: true,
  },
  {
    id: "config",
    title: "Simplify first-run setup",
    agent: "Claude Code",
    state: "idle",
    time: "Yesterday",
    project: "agmux",
  },
  {
    id: "tests",
    title: "Daemon recovery tests",
    agent: "Codex",
    state: "idle",
    time: "Yesterday",
    project: "agmux",
  },
];
export const files = [
  { path: "src/components/SessionList.tsx", add: 48, del: 12 },
  { path: "src/components/ChatView.tsx", add: 72, del: 8 },
  { path: "src/theme.ts", add: 24, del: 3 },
];
export const source = `import { View, Text, Pressable } from 'react-native';
import { useSessions } from '../sessions';
import { theme } from '../theme';

export function SessionList() {
  const { sessions, selected, select } = useSessions();

  return (
    <View style={theme.sidebar}>
      <Text style={theme.heading}>Working</Text>
      {sessions.map((session) => (
        <Pressable
          key={session.id}
          onPress={() => select(session.id)}
          style={[
            theme.session,
            selected === session.id && theme.active,
          ]}
        >
          <Text>{session.title}</Text>
          <StatusDot state={session.state} />
        </Pressable>
      ))}
    </View>
  );
}`;
export const diffLines: {
  kind: "context" | "add" | "del" | "hunk";
  text: string;
  old?: number;
  next?: number;
}[] = [
  { kind: "hunk", text: "@@ -1,6 +1,8 @@" },
  {
    kind: "context",
    old: 1,
    next: 1,
    text: "import { View, Text } from 'react-native';",
  },
  { kind: "del", old: 2, text: "import { Session } from '../types';" },
  { kind: "add", next: 2, text: "import { Pressable } from 'react-native';" },
  { kind: "add", next: 3, text: "import { useSessions } from '../sessions';" },
  {
    kind: "context",
    old: 3,
    next: 4,
    text: "import { theme } from '../theme';",
  },
  { kind: "context", old: 4, next: 5, text: "" },
  { kind: "del", old: 5, text: "export function SessionList({ sessions }) {" },
  { kind: "add", next: 6, text: "export function SessionList() {" },
  { kind: "add", next: 7, text: "  const { sessions, selected, select } =" },
  { kind: "add", next: 8, text: "    useSessions();" },
  { kind: "context", old: 6, next: 9, text: "  return (" },
  {
    kind: "context",
    old: 7,
    next: 10,
    text: "    <View style={theme.sidebar}>",
  },
  { kind: "hunk", text: "@@ -12,5 +15,12 @@" },
  {
    kind: "context",
    old: 12,
    next: 15,
    text: "      {sessions.map((session) => (",
  },
  { kind: "del", old: 13, text: "        <View key={session.id}>" },
  { kind: "add", next: 16, text: "        <Pressable" },
  { kind: "add", next: 17, text: "          key={session.id}" },
  {
    kind: "add",
    next: 18,
    text: "          onPress={() => select(session.id)}",
  },
  { kind: "add", next: 19, text: "          style={[" },
  { kind: "add", next: 20, text: "            theme.session," },
  { kind: "add", next: 21, text: "            selected === session.id" },
  { kind: "add", next: 22, text: "              && theme.active," },
  { kind: "add", next: 23, text: "          ]}" },
  { kind: "add", next: 24, text: "        >" },
  {
    kind: "context",
    old: 14,
    next: 25,
    text: "          <Text>{session.title}</Text>",
  },
  {
    kind: "add",
    next: 26,
    text: "          <StatusDot state={session.state} />",
  },
  { kind: "del", old: 15, text: "        </View>" },
  { kind: "add", next: 27, text: "        </Pressable>" },
  { kind: "context", old: 16, next: 28, text: "      ))}" },
];
