import { test, expect, type Page } from "@playwright/test";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
async function api(page: Page, route: string, body?: unknown) {
  return page.evaluate(
    async ({ route, body }) => {
      const r = await fetch(route, {
        method: body === undefined ? "GET" : "POST",
        headers: {
          "X-Agmux-Token": window.__AGMUX__!.token,
          "Content-Type": "application/json",
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      return { status: r.status, value: await r.json() };
    },
    { route, body },
  );
}
async function start(page: Page, title: string, cwd?: string) {
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await page.getByLabel("Session name").fill(title);
  if (cwd) await page.getByLabel("Project directory").fill(cwd);
  await page.getByLabel("Agent", { exact: true }).selectOption("gemini");
  await page
    .getByRole("button", { name: "Start session", exact: true })
    .click();
  await expect(page.locator("h1")).toHaveText(title);
  await expect(page.locator(".session-item.selected")).toBeInViewport();
  await expect(
    page.getByLabel("Gemini conversation", { exact: true }),
  ).toBeVisible();
  return (await api(page, "/api/snapshot")).value.selected as string;
}
async function send(page: Page, text: string) {
  await page
    .getByLabel("Message Gemini", {
      exact: true,
    })
    .fill(text);
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
}
const messages = (page: Page) =>
  page.getByRole("log", { name: "Conversation messages" });
const review = (page: Page) => page.getByRole("complementary").last();

test("TEST-VER-UX-113/114: real native terminal and proper Chat synchronize one saved conversation", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Shared native conversation");
  await send(page, "remember: amber-42");
  const first = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  await page
    .getByLabel("Message Gemini", { exact: true })
    .fill("unsent Chat draft");
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(
    page.getByLabel("Message Gemini", { exact: true }),
  ).not.toBeVisible();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  const project=(await api(page,"/api/info")).value.cwd;
  await expect.poll(async()=>{try{return await readFile(path.join(path.dirname(project),"gemini-home","terminal-reports.jsonl"),"utf8");}catch{return "";}}).toContain(first.provider_session_id);
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.status,
    )
    .toBe("idle");
  const terminal = page.locator(".xterm-helper-textarea");
  await terminal.pressSequentially("recall");
  await terminal.press("Enter");
  await expect
    .poll(async () =>
      (
        await api(page, "/api/chat", { session_id: id, operation: "state" })
      ).value.messages
        .map((m: any) => m.text)
        .join("\n"),
    )
    .toContain("Remembered: remember: amber-42");
  await terminal.pressSequentially("remember: native-73");
  await terminal.press("Enter");
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.status,
    )
    .toBe("idle");
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_native_terminal_verification.png",
    ),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(page.getByLabel("Message Gemini", { exact: true })).toHaveValue(
    "unsent Chat draft",
  );
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.owner,
    )
    .toBe("chat");
  await expect(messages(page)).toContainText("remember: native-73");
  await send(page, "recall");
  await expect(messages(page)).toContainText(
    "remember: amber-42; remember: native-73",
  );
  const last = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  expect(last.provider_session_id).toBe(first.provider_session_id);
  expect(last.messages.filter((m: any) => m.role === "user")).toHaveLength(4);
  const root = (await api(page, "/api/info")).value.cwd;
  const rows = (await readFile(path.join(root, "acp-requests.jsonl"), "utf8"))
    .trim()
    .split("\n")
    .map((line) => JSON.parse(line))
    .filter((r) => r.sessionId === first.provider_session_id);
  expect(rows.some((r) => r.owner === "native")).toBe(true);
  expect(new Set(rows.map((r) => r.pid)).size).toBeGreaterThanOrEqual(3);
  const currentPid = rows.at(-1).pid;
  for (const pid of new Set<number>(rows.map((r) => r.pid))) {
    if (pid === currentPid) continue;
    await expect
      .poll(() => {
        try {
          process.kill(pid, 0);
          return true;
        } catch {
          return false;
        }
      })
      .toBe(false);
  }
  expect(rows.at(-1).history).toEqual([
    "remember: amber-42",
    "recall",
    "remember: native-73",
    "recall",
  ]);
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_native_chat_sync_verification.png",
    ),
    fullPage: true,
  });
});

test("TEST-VER-UX-114: native turn and approval stay in Terminal until completion", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Native approval ownership");
  await send(page, "remember: ownership");
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.status,
    )
    .toBe("idle");
  await page
    .locator(".xterm-helper-textarea")
    .pressSequentially("native-approval");
  await page.locator(".xterm-helper-textarea").press("Enter");
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(messages(page)).toContainText("native-approval");
  await expect(messages(page)).toContainText("native_edit_tool");
  expect(
    (await api(page, "/api/chat", { session_id: id, operation: "state" })).value
      .owner,
  ).toBe("terminal");
  await expect(
    page.getByRole("button", { name: "Stop response", exact: true }),
  ).toBeVisible();
  expect(
    (
      await api(page, "/api/chat", {
        session_id: id,
        operation: "send",
        message: "must not compete",
        request_id: "native-competing",
      })
    ).status,
  ).not.toBe(200);
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await page.locator(".xterm-helper-textarea").pressSequentially("y");
  await page.locator(".xterm-helper-textarea").press("Enter");
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(messages(page)).toContainText("Permission allowed");
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.owner,
    )
    .toBe("chat");
});

test("TEST-VER-UX-113: failed resume preserves history and never forks", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Load failure");
  await send(page, "remember: retained identity");
  const before = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.status,
    )
    .toBe("idle");
  const root = (await api(page, "/api/info")).value.cwd;
  await writeFile(path.join(root, "fail-session-load"), "1");
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(page.getByRole("alert").last()).toContainText(
    "No replacement conversation was created",
  );
  const after = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  expect(after.provider_session_id).toBe(before.provider_session_id);
  expect(after.messages.map((m: any) => m.text).join(" ")).toContain(
    "retained identity",
  );
  await writeFile(path.join(root, "fail-session-load"), "");
  const { unlink } = await import("node:fs/promises");
  await unlink(path.join(root, "fail-session-load"));
});

test("TEST-VER-UX-115: real applied changes refresh without clicking Refresh", async ({
  page,
}) => {
  await page.goto("/");
  await start(page, "Live file changes");
  await send(page, "write-file");
  await expect(
    review(page).getByText("agent-new.txt", { exact: true }),
  ).toBeVisible();
  await expect(review(page)).toContainText("Created by provider");
  const cwd = (await api(page, "/api/info")).value.cwd;
  await writeFile(path.join(cwd, "README.md"), "# External live change\n");
  await expect(review(page)).toContainText("External live change");
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_live_changes_verification.png",
    ),
    fullPage: true,
  });
});

test("TEST-VER-UX-116/117: proposed before/after diffs, reject, then real applied edit", async ({
  page,
}) => {
  await page.goto("/");
  await start(page, "Edit approval");
  const cwd = (await api(page, "/api/info")).value.cwd;
  const before = await readFile(path.join(cwd, "README.md"), "utf8");
  await page.getByLabel("Message Gemini").fill("propose-change");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  const proposed = review(page).getByRole("region", {
    name: "Proposed changes",
  });
  await expect(proposed).toContainText("Approved change");
  await expect(proposed).toContainText(before.trim());
  expect(await readFile(path.join(cwd, "README.md"), "utf8")).toBe(before);
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Return to Chat" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Return to Chat" }).click();
  await expect(page.getByLabel("Conversation status")).toHaveText("permission");
  await expect(proposed).toBeVisible();
  const panelBox = await review(page).boundingBox(),
    afterBox = await proposed.locator(".split-head span").last().boundingBox();
  expect(afterBox!.x + afterBox!.width).toBeLessThanOrEqual(
    panelBox!.x + panelBox!.width,
  );
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_proposed_verification.png",
    ),
    fullPage: true,
  });
  await proposed.getByRole("button", { name: "Reject", exact: true }).click();
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
  expect(await readFile(path.join(cwd, "README.md"), "utf8")).toBe(before);
  await expect(proposed).toHaveCount(0);
  await page
    .getByLabel("Message Gemini", { exact: true })
    .fill("propose-change");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(proposed).toBeVisible();
  await proposed
    .getByRole("button", { name: "Allow once", exact: true })
    .click();
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
  await expect
    .poll(() => readFile(path.join(cwd, "README.md"), "utf8"))
    .toBe("# Approved change\n");
  await expect(proposed).toHaveCount(0);
  await expect(review(page)).toContainText("Approved change");
});

test("TEST-VER-UX-118/124: end → Inactive → Archive retains readable history", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Retained session");
  await send(page, "remember: retained-value");
  await page.getByRole("button", { name: "End session", exact: true }).click();
  await expect(
    page.getByText("Inactive", { exact: true }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Archive session", exact: true }),
  ).toBeVisible();
  await expect(messages(page)).toContainText("remember: retained-value");
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeDisabled();
  const restartRoot = path.dirname((await api(page, "/api/info")).value.cwd);
  await writeFile(path.join(restartRoot, "restart-request"), "restart");
  await expect
    .poll(
      async () => {
        try {
          return await readFile(
            path.join(restartRoot, "restart-complete"),
            "utf8",
          );
        } catch {
          return "";
        }
      },
      { timeout: 15000 },
    )
    .not.toBe("");
  await expect
    .poll(
      async () => {
        try {
          return (await api(page, "/api/snapshot")).status;
        } catch {
          return 0;
        }
      },
      { timeout: 15000 },
    )
    .toBe(200);
  await page.reload();
  await expect(messages(page)).toContainText("remember: retained-value");
  expect(
    (
      await api(page, "/api/chat", {
        session_id: id,
        operation: "send",
        message: "must not restart",
        request_id: "ended-send",
      })
    ).status,
  ).toBeGreaterThanOrEqual(400);
  expect(
    (await api(page, "/api/chat", { session_id: id, operation: "state" })).value
      .status,
  ).toBe("ended");
  await page
    .getByRole("button", { name: "Archive session", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Restore from archive", exact: true }),
  ).toBeVisible();
  await page.reload();
  expect((await api(page, "/api/snapshot")).value.metadata[id].archived).toBe(
    true,
  );
  await expect(messages(page)).toContainText("remember: retained-value");
  await page.getByRole("button", { name: "Archived (1)", exact: true }).click();
  await expect(
    page.locator(".session-item").filter({ hasText: "Retained session" }),
  ).toBeVisible();
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_inactive_verification.png",
    ),
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "Restore from archive", exact: true })
    .click();
  expect((await api(page, "/api/snapshot")).value.metadata[id].archived).toBe(
    false,
  );
});

test("TEST-VER-UX-119/120: canonical project groups, recent activity and single-line compact rows", async ({
  page,
}) => {
  await page.goto("/");
  const root = path.dirname((await api(page, "/api/info")).value.cwd);
  const a = path.join(root, "a", "shared"),
    b = path.join(root, "b", "shared");
  await mkdir(a, { recursive: true });
  await mkdir(b, { recursive: true });
  const idA = await start(page, "Project A", a);
  await start(page, "Project B", b);
  await page.getByRole("button", { name: "Projects", exact: true }).click();
  await expect(
    page.locator(".group-heading").filter({ hasText: "/a/shared" }),
  ).toBeVisible();
  await expect(
    page.locator(".group-heading").filter({ hasText: "/b/shared" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Projects", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await api(page, "/api/chat", {
    session_id: idA,
    operation: "send",
    message: "latest activity",
    request_id: "project-activity",
  });
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await expect(page.locator(".session-item").first()).toContainText(
    "Project A",
  );
  await page.getByRole("button", { name: "Compact session rows" }).click();
  if (
    (await page
      .getByRole("button", { name: "Compact session rows" })
      .getAttribute("aria-pressed")) !== "true"
  )
    await page.getByRole("button", { name: "Compact session rows" }).click();
  expect(
    await page
      .locator(".session-item")
      .first()
      .evaluate((el) => el.getBoundingClientRect().height),
  ).toBeLessThanOrEqual(36);
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_activity_verification.png",
    ),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Projects", exact: true }).click();
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_projects_verification.png",
    ),
    fullPage: true,
  });
  // Logical idle agents do not launch provider processes; stress the real list.
  const title = "Long project session " + "x".repeat(80);
  for (let index = 0; index < 48; index++) {
    expect(
      (
        await api(page, "/api/spawn", {
          agent: "gemini",
          cwd: b,
          title: index === 47 ? title : `Compact agent ${index}`,
          args: [],
        })
      ).status,
    ).toBe(200);
  }
  await page.reload();
  await expect(page.locator("h1")).toHaveText(title);
  const selected = page.locator(".session-item.selected");
  await expect(selected).toHaveAttribute("title", new RegExp(title));
  await selected.focus();
  await selected.press("Enter");
  await page.locator(".virtual-sessions").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await expect
    .poll(() => page.locator(".session-item").count())
    .toBeLessThan(40);
  await expect
    .poll(async () =>
      page.locator(".session-item").evaluateAll((items) => {
        const boxes = items
          .map((el) => el.getBoundingClientRect())
          .sort((a, b) => a.top - b.top);
        return boxes.every(
          (box, index) =>
            box.height <= 36 &&
            (index === 0 || box.top >= boxes[index - 1].bottom),
        );
      }),
    )
    .toBe(true);
});

test("TEST-VER-UX-121: no Shell option/default or fallback without installed agents", async ({
  page,
}) => {
  await page.goto("/");
  const info = (await api(page, "/api/info")).value;
  await page.route("**/api/info", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        ...info,
        agents: info.agents.map((a: object) => ({ ...a, available: false })),
      }),
    }),
  );
  await page.reload();
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await expect(
    page.getByLabel("Agent", { exact: true }).locator('option[value="shell"]'),
  ).toHaveCount(0);
  expect(await page.getByLabel("Agent", { exact: true }).inputValue()).not.toBe(
    "shell",
  );
  await expect(
    page.getByRole("button", { name: "Start session", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText("No agent provider is installed.", { exact: false }),
  ).toBeVisible();
});

test("TEST-VER-UX-122: palette, quick switch, shortcuts, focus and composition", async ({
  page,
}) => {
  await page.goto("/");
  await start(page, "Keyboard session");
  const prompt = page.getByLabel("Message Gemini", { exact: true });
  await prompt.fill("keep this draft");
  await prompt.focus();
  await prompt.press("Control+Shift+P");
  await expect(
    page.getByRole("dialog", { name: "Command palette", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(prompt).toBeFocused();
  await expect(prompt).toHaveValue("keep this draft");
  await prompt.press("F1");
  await expect(
    page.getByRole("dialog", { name: "Command palette", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(prompt).toBeFocused();
  await prompt.dispatchEvent("keydown", {
    key: "p",
    ctrlKey: true,
    shiftKey: true,
    isComposing: true,
  });
  await expect(
    page.getByRole("dialog", { name: "Command palette", exact: true }),
  ).toHaveCount(0);
  await prompt.press("Control+Shift+P");
  const commandSearch = page.getByLabel("Find a command");
  await commandSearch.fill("> psv");
  await expect(
    page.getByRole("option", { name: "Project session view", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await commandSearch.dispatchEvent("keydown", {
    key: "Enter",
    isComposing: true,
  });
  await expect(commandSearch).toBeFocused();
  await commandSearch.press("Enter");
  await expect(
    page.getByRole("button", { name: "Projects", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Control+p");
  await expect(
    page.getByRole("dialog", { name: "Quick session switcher" }),
  ).toBeVisible();
  await page.getByLabel("Find a session or project").fill("Keyboard session");
  await page.keyboard.press("Enter");
  await expect(page.locator("h1")).toHaveText("Keyboard session");
  await prompt.focus();
  await prompt.press("Control+k");
  await prompt.press("Control+s");
  await expect(
    page.getByRole("dialog", { name: "Keyboard shortcuts", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_keyboard_verification.png",
    ),
    fullPage: true,
  });
  await page.keyboard.press("Escape");
  await prompt.press("Control+Backquote");
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(prompt).toHaveValue("keep this draft");
});

test("TEST-VER-UX-113: Terminal-first empty session continues in proper Chat without changing UUID", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Terminal first");
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.status,
    )
    .toBe("idle");
  const first = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value.provider_session_id;
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await send(page, "remember: native first");
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.owner,
    )
    .toBe("chat");
  await send(page, "recall");
  await expect(messages(page)).toContainText(
    "Remembered: remember: native first",
  );
  expect(
    (await api(page, "/api/chat", { session_id: id, operation: "state" })).value
      .provider_session_id,
  ).toBe(first);
});

test("TEST-VER-UX-113: native CLI exit is reported and Chat resumes the same session", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Native exit");
  await send(page, "remember: native exit");
  const uuid = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value.provider_session_id;
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await page.locator(".xterm-helper-textarea").pressSequentially("/quit");
  await page.locator(".xterm-helper-textarea").press("Enter");
  await expect(page.locator(".terminal-footer")).toContainText(
    "Process exited (0)",
  );
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await api(page, "/api/chat", { session_id: id, operation: "state" }))
          .value.owner,
    )
    .toBe("chat");
  expect(
    (await api(page, "/api/chat", { session_id: id, operation: "state" })).value
      .provider_session_id,
  ).toBe(uuid);
  await expect(messages(page)).toContainText("remember: native exit");
});

test("TEST-VER-UX-114: rapid tab changes preserve the latest interface owner", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Rapid view switching");
  await send(page, "remember: latest view");
  let delayed = false;
  await page.route("**/api/chat", async (route) => {
    const body = route.request().postDataJSON();
    if (body.operation === "view" && body.view === "terminal" && !delayed) {
      delayed = true;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    await route.continue();
  });
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect.poll(() => delayed).toBe(true);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect
    .poll(async () => {
      const state = (
        await api(page, "/api/chat", { session_id: id, operation: "state" })
      ).value;
      return state.owner + ":" + state.view;
    })
    .toBe("chat:chat");
  await expect(
    page.getByLabel("Message Gemini", { exact: true }),
  ).toBeVisible();
  await expect(page.locator(".xterm-screen")).toHaveCount(0);
  await expect(messages(page)).toContainText("remember: latest view");
});
