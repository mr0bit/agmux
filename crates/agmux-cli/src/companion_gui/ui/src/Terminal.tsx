import { useEffect, useRef } from "react";
import { Terminal as XTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

/** A real terminal renderer with a deliberately local, non-executing demo shell. */
export default function Terminal({
  sessionId,
  light = false,
}: {
  sessionId: string;
  light?: boolean;
}) {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const term = new XTerminal({
      fontSize: 12,
      lineHeight: 1.65,
      fontFamily: "'SFMono-Regular', Consolas, monospace",
      cursorBlink: true,
      theme: light
        ? {
            background: "#faf9f6",
            foreground: "#302d27",
            cursor: "#5d5547",
            selectionBackground: "#dbe5f1",
            green: "#287344",
            blue: "#365caa",
            black: "#25221d",
            brightBlack: "#736b5e",
          }
        : {
            background: "#171615",
            foreground: "#dad7d2",
            cursor: "#aaa499",
            selectionBackground: "#49443d",
            green: "#8dc793",
            blue: "#a8bbf1",
          },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current!);
    const resize = new ResizeObserver(() => {
      if (host.current?.clientWidth) fit.fit();
    });
    resize.observe(host.current!);
    term.writeln("\x1b[90mAGMUX  /  LOCAL DEMO TERMINAL\x1b[0m");
    term.writeln("Session: " + sessionId + "\r\n");
    term.writeln("\x1b[32m❯\x1b[0m cargo check --workspace");
    term.writeln("    \x1b[32mChecking\x1b[0m agmux-proto v0.1.0");
    term.writeln("    \x1b[32mChecking\x1b[0m agmux-core v0.1.0");
    term.writeln("    \x1b[32mChecking\x1b[0m agmux-daemon v0.1.0");
    term.writeln("    \x1b[32mFinished\x1b[0m `dev` profile [demo output]\r\n");
    term.writeln(
      "\x1b[90mUI preview only. Commands do not run on your machine.\x1b[0m",
    );
    term.writeln("Try: help, sessions, pwd, clear\r\n");
    const prompt = () => term.write("\x1b[32m❯\x1b[0m ");
    prompt();
    let line = "";
    const input = term.onData((data) => {
      for (const char of data) {
        if (char === "\r") {
          term.write("\r\n");
          const cmd = line.trim();
          if (cmd === "clear") term.clear();
          else if (cmd === "help")
            term.writeln("Local preview: help · sessions · pwd · clear");
          else if (cmd === "pwd") term.writeln("/workspace/agmux");
          else if (cmd === "sessions")
            term.writeln(
              "workspace  claude  waiting\r\nstream     codex   working\r\nterminal   opencode working",
            );
          else if (cmd)
            term.writeln(
              "Demo terminal: " +
                cmd +
                "\r\nConnect the Rust PTY bridge to execute commands.",
            );
          line = "";
          prompt();
        } else if (char === "\x7f") {
          if (line) {
            line = line.slice(0, -1);
            term.write("\b \b");
          }
        } else if (char >= " " && char !== "\x7f") {
          line += char;
          term.write(char);
        }
      }
    });
    return () => {
      input.dispose();
      resize.disconnect();
      term.dispose();
    };
  }, [sessionId, light]);
  return (
    <div className="terminal-host" ref={host} aria-label="Demo terminal" />
  );
}
