import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import type { Backend } from "./backend";

export default function LiveTerminal({
  backend,
  sessionId,
  light,
}: {
  backend: Backend;
  sessionId: string;
  light: boolean;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState("Connecting terminal…");
  useEffect(() => {
    let disposed = false,
      retry = 0,
      socket: WebSocket | null = null,
      failures = 0,
      sync = false;
    const term = new Terminal({
      cursorBlink: !matchMedia("(prefers-reduced-motion: reduce)").matches,
      fontSize: 13,
      lineHeight: 1.45,
      fontFamily: "SFMono-Regular, Consolas, monospace",
      scrollback: 10000,
      theme: light
        ? {
            background: "#fffefb",
            foreground: "#302c26",
            cursor: "#51432f",
            selectionBackground: "#dce5f0",
            brightBlack: "#776d60",
          }
        : {
            background: "#171614",
            foreground: "#f0ede7",
            cursor: "#d5c7b0",
            selectionBackground: "#484c61",
            brightBlack: "#ada498",
          },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current!);
    const resize = () => {
      if (!host.current?.clientWidth || !host.current?.clientHeight) return;
      fit.fit();
      if (socket?.readyState === WebSocket.OPEN)
        socket.send(
          JSON.stringify({
            type: "resize",
            cols: Math.max(2, Math.min(500, term.cols)),
            rows: Math.max(2, Math.min(200, term.rows)),
          }),
        );
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host.current!);
    const input = term.onData((data) => {
      if (socket?.readyState === WebSocket.OPEN)
        socket.send(new TextEncoder().encode(data));
    });
    const connect = () => {
      if (disposed) return;
      socket = backend.terminal(sessionId);
      socket.binaryType = "arraybuffer";
      socket.onopen = () => {
        failures = 0;
        setStatus("Terminal connected");
        resize();
      };
      socket.onmessage = (event) => {
        if (typeof event.data === "string") {
          try {
            const frame = JSON.parse(event.data);
            if (frame.type === "sync") {
              sync = true;
              term.reset();
              term.resize(frame.cols, frame.rows);
            }
            if (frame.type === "exit") {
              setStatus(`Process exited (${frame.code ?? "unknown"})`);
              socket?.close();
              disposed = true;
            }
          } catch {
            setStatus("Invalid terminal frame");
          }
        } else
          term.write(new Uint8Array(event.data), () => {
            if (sync) {
              sync = false;
              resize();
            }
          });
      };
      socket.onclose = () => {
        if (!disposed) {
          setStatus("Terminal disconnected · reconnecting…");
          retry = window.setTimeout(
            connect,
            Math.min(5000, 500 * 2 ** failures++),
          );
        }
      };
      socket.onerror = () =>
        setStatus("Terminal connection failed · retrying…");
    };
    connect();
    return () => {
      disposed = true;
      clearTimeout(retry);
      socket?.close();
      observer.disconnect();
      input.dispose();
      term.dispose();
    };
  }, [backend, sessionId, light]);
  return (
    <div className="live-terminal">
      <div
        ref={host}
        className="live-terminal-host"
        aria-label="Live terminal"
      />
      <div className="terminal-footer" role="status">
        {status} · keyboard input goes to this session
      </div>
    </div>
  );
}
