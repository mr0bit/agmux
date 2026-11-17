export type NativeSession = {
  session_id: string;
  command: string;
  cwd: string;
  pid: number;
  state: string;
  created_at: number;
};
export type Metadata = {
  project: string;
  group: string;
  favorite: boolean;
  archived: boolean;
};
export type Panel = {
  id: number;
  session_id: string;
  target: string;
  kind: string;
};
export type Snapshot = {
  chat_states?: Record<string,string>;
  sessions: NativeSession[];
  metadata: Record<string, Metadata>;
  selected: string | null;
  screen: string;
  panels: Panel[];
  attention: number;
  notifications: { title: string; body: string; session_id: string }[];
};
export type MachineInfo = {
  version: string;
  machine: string;
  cwd: string;
  agents: { id: string; name: string; command: string; available: boolean }[];
  capabilities: Record<string, boolean>;
};
export type NativeDiff = {
  old_path: string | null;
  new_path: string | null;
  hunks: {
    header: string;
    lines: {
      kind: "Context" | "Added" | "Removed" | "NoNewline";
      content: string;
      old_line: number | null;
      new_line: number | null;
    }[];
  }[];
};
declare global {
  interface Window {
    __AGMUX__?: { token: string; version: number };
  }
}
/** Browser-process secret stays in memory and is never the daemon credential. */
export class Backend {
  constructor(private readonly token: string) {}
  async request<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(path, {
      method: body === undefined ? "GET" : "POST",
      headers: {
        "X-Agmux-Token": this.token,
        ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(path === "/api/models" ? 105000 : 15000),
      cache: "no-store",
    });
    const value = await response.json();
    if (!response.ok)
      throw new Error(value.error || `Request failed (${response.status})`);
    return value as T;
  }
  events(sessionId?: string) {
    return new EventSource(
      `/api/events?token=${encodeURIComponent(this.token)}${sessionId ? "&session_id=" + encodeURIComponent(sessionId) : ""}`,
    );
  }
  terminal(sessionId: string) {
    return new WebSocket(
      `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/api/terminal?token=${encodeURIComponent(this.token)}&session_id=${encodeURIComponent(sessionId)}`,
    );
  }
}
