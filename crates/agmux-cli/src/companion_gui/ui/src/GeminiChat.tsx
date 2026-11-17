import { useEffect, useRef, useState } from "react";
import { ArrowUp, Square, Sparkles } from "lucide-react";
import type { Backend } from "./backend";
type ChatState = {
  messages: {id: string; role: string; text: string; status?: string}[];
  status: string; error: string | null;
  permission: {id: unknown; title: string; options: {optionId: string; name: string; kind: string}[]} | null;
};
const initial: ChatState = {messages: [], status: "idle", error: null, permission: null};
export default function GeminiChat({backend, sessionId, connected, onStatus}: {
  backend: Backend; sessionId: string; connected: boolean; onStatus: (status: string) => void;
}) {
  const [chat, setChat] = useState(initial), [draft, setDraft] = useState(""), [error, setError] = useState(""), [sending, setSending] = useState(false);
  const pending = useRef<{id: string; text: string} | null>(null), list = useRef<HTMLDivElement>(null), follow = useRef(true);
  const statusRef = useRef(onStatus); statusRef.current = onStatus;
  useEffect(() => {
    let disposed = false, timer = 0;
    const poll = async () => {
      try {
        const state = await backend.request<ChatState>("/api/chat", {session_id: sessionId, operation: "state"});
        if (!disposed) {setChat(state); statusRef.current(state.status);}
      } catch (e) {if (!disposed) setError(e instanceof Error ? e.message : String(e));}
      finally {if (!disposed) timer = window.setTimeout(poll, 700);}
    };
    void poll(); return () => {disposed = true; clearTimeout(timer);};
  }, [backend, sessionId]);
  useEffect(() => {if (follow.current && list.current) list.current.scrollTop = list.current.scrollHeight;}, [chat.messages]);
  const active = ["working", "permission", "cancelling"].includes(chat.status);
  const operation = async (operation: string, fields: Record<string, unknown> = {}) => {
    try {setError(""); const state = await backend.request<ChatState>("/api/chat", {session_id: sessionId, operation, ...fields}); setChat(state); statusRef.current(state.status); return true;}
    catch (e) {setError(e instanceof Error ? e.message : String(e)); return false;}
  };
  const send = async () => {
    if (sending || active || !draft.trim()) return;
    setSending(true);
    // Retrying an ambiguous HTTP failure reuses the same daemon request ID.
    if (pending.current?.text !== draft) pending.current = {id: crypto.randomUUID(), text: draft};
    if (await operation("send", {message: draft, request_id: pending.current!.id})) {setDraft(""); pending.current = null;}
    setSending(false);
  };
  return <section className="gemini-chat" aria-label="Gemini conversation">
    <div className="live-chat-header"><Sparkles size={16}/><strong>Gemini</strong><span role="status" aria-label="Conversation status">{sending ? "Submitting…" : chat.status}</span></div>
    <p className="live-hint">Chat is a Gemini conversation in this project. Terminal runs an independent native CLI. History stays available while the daemon is running.</p>
    <div className="conversation-messages" ref={list} role="log" aria-label="Conversation messages" onScroll={() => {const el=list.current!; follow.current=el.scrollHeight-el.scrollTop-el.clientHeight<80;}}>
      {!chat.messages.length && <div className="conversation-empty"><Sparkles size={26}/><h2>What shall we work on?</h2><p>Start a conversation with Gemini in your project.</p></div>}
      {chat.messages.map(message => <article key={message.id} className={`conversation-message ${message.role}`}><strong>{message.role === "user" ? "You" : message.role === "tool" ? "Tool" : "Gemini"}</strong><div>{message.text}</div>{message.status && <small>{message.status}</small>}</article>)}
      {active && <div className="conversation-progress" role="status">{chat.status === "permission" ? "Waiting for your approval" : chat.status === "cancelling" ? "Stopping…" : "Gemini is working…"}</div>}
    </div>
    {chat.permission && <div className="conversation-permission" role="group" aria-label="Tool permission"><strong>{chat.permission.title || "Gemini requests permission"}</strong><div>{chat.permission.options.map(option => <button className="live-action" key={option.optionId} onClick={() => void operation("permission", {id: chat.permission!.id, option_id: option.optionId})}>{option.name}</button>)}</div></div>}
    {(chat.error || error) && <div className="conversation-error" role="alert"><strong>{(chat.error || error).split("\n")[0].slice(0,200)}</strong><details><summary>Details</summary><pre>{chat.error || error}</pre></details></div>}
    <form className="conversation-composer" onSubmit={event => {event.preventDefault(); void send();}}>
      <textarea aria-label="Message Gemini" placeholder="Ask Gemini about this project…" value={draft} onChange={event => setDraft(event.target.value)} disabled={!connected || sending} onKeyDown={event => {if(event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing){event.preventDefault();void send();}}}/>
      {active ? <button className="live-action" type="button" aria-label="Stop response" disabled={chat.status === "cancelling"} onClick={() => void operation("cancel")}><Square size={16}/></button> : <button className="primary-button" aria-label="Send message" disabled={!connected || sending || !draft.trim() || !["idle", "cancelled"].includes(chat.status)}><ArrowUp size={16}/></button>}
      <small>Enter to send · Shift+Enter for a new line</small>
    </form>
  </section>;
}
