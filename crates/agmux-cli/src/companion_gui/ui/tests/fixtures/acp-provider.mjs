// External provider stand-in: protocol/error tests only, never model acceptance.
import readline from "node:readline";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
const home = process.env.GEMINI_CLI_HOME || process.cwd();
const journal = (id) => path.join(home, "test-chats", id + ".jsonl");
const records = (id) => [
  ...new Map(
    fs
      .readFileSync(journal(id), "utf8")
      .trim()
      .split("\n")
      .slice(1)
      .map((line) => JSON.parse(line))
      .map((item) => [item.id, item]),
  ).values(),
];
const append = (id, value) =>
  fs.appendFileSync(journal(id), JSON.stringify(value) + "\n");
const initialize = (id) => {
  fs.mkdirSync(path.dirname(journal(id)), { recursive: true });
  fs.writeFileSync(journal(id), JSON.stringify({ sessionId: id }) + "\n");
};
const hook = (id, event) => {
  const settings = JSON.parse(
    fs.readFileSync(process.env.GEMINI_CLI_SYSTEM_DEFAULTS_PATH, "utf8"),
  );
  if (
    settings.general?.preferredEditor !== "code" ||
    !settings.hooks.SessionStart.some((group) =>
      group.hooks.some((h) => h.name === "user-default-hook"),
    )
  )
    throw Error("Existing provider defaults/hooks were lost");
  for (const group of settings.hooks[event] || [])
    for (const config of group.hooks) {
      const executable =
        process.platform === "win32" ? "powershell.exe" : "bash";
      const args =
        process.platform === "win32"
          ? ["-NoProfile", "-NonInteractive", "-Command", config.command]
          : ["-c", config.command];
      execFileSync(executable, args, {
        input: JSON.stringify({
          hook_event_name: event,
          session_id: id,
          transcript_path: journal(id),
        }),
        env: { ...process.env, ...config.env },
        stdio: ["pipe", "pipe", "pipe"],
      });
    }
};

const acp = process.argv.includes("--acp");
if (
  !acp &&
  (process.argv.includes("--resume") || process.argv.includes("--session-id"))
) {
  const resumed = process.argv.indexOf("--resume"),
    at = resumed >= 0 ? resumed : process.argv.indexOf("--session-id");
  const id = process.argv[at + 1];
  if (resumed < 0) initialize(id);
  if (!fs.existsSync(journal(id)))
    throw Error("Saved session missing; must not create a replacement");
  process.stdout.write("\x1b[?2004hNative Gemini CLI fixture · " + id + "\r\n");
  for (const item of records(id))
    process.stdout.write(
      (item.type === "user" ? "You: " : "Gemini: ") + item.content + "\r\n",
    );
  hook(id, "SessionStart");
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  let input = "",
    turn = records(id).filter((m) => m.type === "user").length,
    waiting = false;
  const complete = (text) => {
    if (waiting)
      append(id, {
        id: "native-tool-" + turn,
        type: "gemini",
        content: "",
        toolCalls: [
          { id: "edit-" + turn, name: "native_edit_tool", status: "completed" },
        ],
      });
    append(id, {
      id: "native-assistant-" + turn,
      type: "gemini",
      content: text,
    });
    process.stdout.write("Gemini: " + text + "\r\n> ");
    hook(id, "AfterAgent");
    waiting = false;
  };
  process.stdin.on("data", (data) => {
    input += data.toString().replace(/\x1b\[20[01]~/g, "");
    if (input.includes("\x03")) {
      input = "";
      complete("Cancelled");
      return;
    }
    let enter;
    while ((enter = input.indexOf("\r")) >= 0) {
      const text = input.slice(0, enter).replace(/\n/g, "");
      input = input.slice(enter + 1);
      if (text === "/quit") process.exit(0);
      if (waiting) {
        complete(text === "y" ? "Permission allowed" : "Permission rejected");
        continue;
      }
      turn++;
      append(id, { id: "native-user-" + turn, type: "user", content: text });
      hook(id, "BeforeAgent");
      const history = records(id)
        .filter((m) => m.type === "user")
        .map((m) => m.content);
      fs.appendFileSync(
        "acp-requests.jsonl",
        JSON.stringify({
          pid: process.pid,
          sessionId: id,
          history,
          owner: "native",
          text,
        }) + "\n",
      );
      process.stdout.write("You: " + text + "\r\n");
      if (text === "native-approval") {
        waiting = true;
        append(id, {
          id: "native-tool-" + turn,
          type: "gemini",
          content: "",
          toolCalls: [
            {
              id: "edit-" + turn,
              name: "native_edit_tool",
              status: "awaiting_approval",
            },
          ],
        });
        process.stdout.write("Approve native tool? y/n\r\n");
      } else if (text === "wait-for-cancel") {
        waiting = true;
      } else
        setTimeout(
          () =>
            complete(
              text === "recall"
                ? "Remembered: " +
                    history.filter((t) => t.startsWith("remember:")).join("; ")
                : "Native reply: " + text,
            ),
          150,
        );
    }
  });
} else if (!acp) {
  if (process.argv.includes("--version")) {
    console.log("ACP test fixture (not Gemini)");
    process.exit(0);
  }
  process.stdout.write(
    "\x1b[?2004h┌──────────┐\r\n│ Terminal │\r\n└──────────┘\r\nNative CLI fixture only\r\n",
  );
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  let input = Buffer.alloc(0);
  process.stdin.on("data", (data) => {
    input = Buffer.concat([input, Buffer.from(data)]);
    let enter;
    while ((enter = input.indexOf(13)) >= 0) {
      process.stdout.write(
        `INPUT:${input.subarray(0, enter + 1).toString("hex")}\r\n`,
      );
      input = input.subarray(enter + 1);
    }
  });
} else {
  let turn = 0,
    active = null,
    permission = null,
    ignoreCancel = false,
    pendingEdit = null;
  let sessionId = `fixture-${process.pid}`;
  let history = [];
  const output = (value) =>
    process.stdout.write(JSON.stringify({ jsonrpc: "2.0", ...value }) + "\n");
  const respond = (id, result) => output({ id, result });
  let assistant = "";
  const chunk = (text) => {
    assistant += text;
    append(sessionId, {
      id: "acp-assistant-" + turn,
      type: "gemini",
      content: assistant,
    });
    output({
      method: "session/update",
      params: {
        sessionId: sessionId,
        update: {
          sessionUpdate: "agent_message_chunk",
          content: { type: "text", text },
        },
      },
    });
  };
  const finish = (reason = "end_turn") => {
    assistant = "";
    if (active !== null) {
      respond(active, { stopReason: reason });
      active = null;
    }
  };
  readline.createInterface({ input: process.stdin }).on("line", (line) => {
    const req = JSON.parse(line);
    if (req.method === "initialize") {
      if (!fs.existsSync("hang-initialize"))
        respond(req.id, {
          protocolVersion: 1,
          agentCapabilities: { loadSession: !fs.existsSync("no-session-load") },
          authMethods: [],
        });
    } else if (req.method === "session/load") {
      sessionId = req.params.sessionId;
      if (fs.existsSync("fail-session-load")) {
        output({
          id: req.id,
          error: { code: -32000, message: "Saved session unavailable" },
        });
        return;
      }
      const items = records(sessionId);
      history = items.filter((m) => m.type === "user").map((m) => m.content);
      turn = history.length;
      for (const item of items)
        output({
          method: "session/update",
          params: {
            sessionId,
            update: {
              sessionUpdate:
                item.type === "user"
                  ? "user_message_chunk"
                  : "agent_message_chunk",
              content: { type: "text", text: item.content },
            },
          },
        });
      for (const item of items)
        for (const call of item.toolCalls || [])
          output({
            method: "session/update",
            params: {
              sessionId,
              update: {
                sessionUpdate: "tool_call",
                toolCallId: call.id,
                title: call.name,
                status: call.status,
              },
            },
          });
      respond(req.id, {});
    } else if (req.method === "session/new") {
      initialize(sessionId);
      if (req.params.cwd.startsWith("\\\\?\\")) {
        output({
          id: req.id,
          error: {
            code: -32602,
            message:
              "Node-facing project paths must not be Windows verbatim paths",
          },
        });
        return;
      }
      const at = process.argv.indexOf("--model");
      if (fs.existsSync("no-model-inventory")) {
        respond(req.id, { sessionId: sessionId });
        return;
      }
      respond(req.id, {
        sessionId: sessionId,
        models: {
          currentModelId: at >= 0 ? process.argv[at + 1] : "provider-default",
          availableModels: [
            { modelId: "provider-default", name: "Provider default" },
            { modelId: "fixture-fast", name: "Fixture fast" },
          ],
        },
      });
    } else if (req.method === "session/prompt") {
      turn++;
      active = req.id;
      const text = req.params.prompt[0].text;
      history.push(text);
      append(sessionId, {
        id: "acp-user-" + turn,
        type: "user",
        content: text,
      });
      fs.appendFileSync(
        "acp-requests.jsonl",
        JSON.stringify({
          turn,
          text,
          args: process.argv,
          pid: process.pid,
          sessionId,
          history: [...history],
        }) + "\n",
      );
      if (text === "write-file") {
        fs.writeFileSync("agent-new.txt", "Created by provider\n");
        chunk("Created agent-new.txt");
        finish();
      } else if (text === "recall") {
        chunk(
          "Remembered: " +
            (history.filter((t) => t.startsWith("remember:")).join("; ") ||
              "no prior fact"),
        );
        finish();
      } else if (text === "propose-change") {
        const file = path.resolve("README.md");
        const oldText = fs.readFileSync(file, "utf8");
        const newText = "# Approved change\n";
        pendingEdit = { file, newText };
        permission = 600;
        output({
          id: permission,
          method: "session/request_permission",
          params: {
            sessionId,
            toolCall: {
              toolCallId: "edit-1",
              title: "Update README.md",
              kind: "edit",
              content: [{ type: "diff", path: file, oldText, newText }],
            },
            options: [
              {
                optionId: "allow-once",
                name: "Allow once",
                kind: "allow_once",
              },
              { optionId: "reject-once", name: "Reject", kind: "reject_once" },
            ],
          },
        });
      } else if (text === "reject-provider") {
        output({
          id: req.id,
          error: { code: -32000, message: "Provider rejected this request" },
        });
        active = null;
      } else if (text === "exit-provider") process.exit(23);
      else if (text === "wait-for-cancel" || text === "ignore-cancel") {
        ignoreCancel = text === "ignore-cancel";
      } else if (text === "ask-permission") {
        permission = 600;
        output({
          id: permission,
          method: "session/request_permission",
          params: {
            sessionId: sessionId,
            toolCall: { toolCallId: "tool-1", title: "Write project file" },
            options: [
              {
                optionId: "allow-once",
                name: "Allow once",
                kind: "allow_once",
              },
              { optionId: "reject-once", name: "Reject", kind: "reject_once" },
            ],
          },
        });
      } else {
        setTimeout(() => {
          chunk(`Response ${turn}: `);
          setTimeout(() => {
            chunk(text);
            finish();
          }, 50);
        }, 80);
      }
    } else if (req.method === "session/cancel") {
      if (!ignoreCancel) finish("cancelled");
    } else if (req.id === permission && req.result) {
      if (pendingEdit && req.result.outcome.optionId === "allow-once")
        fs.writeFileSync(pendingEdit.file, pendingEdit.newText);
      pendingEdit = null;
      chunk(`Permission: ${req.result.outcome.optionId}`);
      permission = null;
      finish();
    } else if (req.id !== undefined)
      output({
        id: req.id,
        error: { code: -32601, message: "Unsupported fixture method" },
      });
  });
}
