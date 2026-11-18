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
async function send(page: Page, text: string, terminal = false) {
  await page
    .getByLabel(terminal ? "Terminal prompt" : "Message Gemini", {
      exact: true,
    })
    .fill(text);
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
}
const messages = (page: Page) =>
  page.getByRole("log", { name: "Conversation messages" });
const review = (page: Page) => page.getByRole("complementary").last();

test("TEST-VER-UX-113/114: one provider context and draft across Chat → Terminal → Chat", async ({
  page,
}) => {
  await page.goto("/");
  const id = await start(page, "Shared conversation");
  await send(page, "remember: amber-42");
  await page.getByLabel("Message Gemini", { exact: true }).fill("unsent draft");
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.getByLabel("Terminal prompt")).toHaveValue("unsent draft");
  await expect(messages(page)).toContainText("remember: amber-42");
  await send(page, "recall", true);
  await expect(messages(page)).toContainText("Remembered: remember: amber-42");
  const first = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(messages(page)).toContainText("Remembered: remember: amber-42");
  await api(page, "/api/reply", {
    session_id: id,
    message: "legacy reply into same context",
  });
  await expect(messages(page)).toContainText("legacy reply into same context");
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
  await api(page, "/api/send", {
    session_id: id,
    message: "delegated prompt into same context",
  });
  await expect(messages(page)).toContainText(
    "delegated prompt into same context",
  );
  await expect(page.getByLabel("Conversation status")).toHaveText("idle");
  const last = (
    await api(page, "/api/chat", { session_id: id, operation: "state" })
  ).value;
  expect(last.provider_session_id).toBe(first.provider_session_id);
  expect(
    last.messages.filter((m: { role: string }) => m.role === "user"),
  ).toHaveLength(4);
  const root = (await api(page, "/api/info")).value.cwd;
  const records = (
    await readFile(path.join(root, "acp-requests.jsonl"), "utf8")
  )
    .trim()
    .split("\n")
    .map((s) => JSON.parse(s));
  const rows = records.filter((r) => r.history?.includes("remember: amber-42"));
  expect(new Set(rows.map((r) => r.pid)).size).toBe(1);
  expect(rows.at(-1).history).toEqual([
    "remember: amber-42",
    "recall",
    "legacy reply into same context",
    "delegated prompt into same context",
  ]);
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await page.screenshot({
    path: path.resolve(
      "../../../../../docs/assets/screenshots/qa17_b2_shared_verification.png",
    ),
    fullPage: true,
  });
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
  await page.getByLabel("Terminal prompt").fill("propose-change");
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
  await page.getByLabel("Find a command").fill("Project session view");
  await page.keyboard.press("Enter");
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
  await expect(page.getByLabel("Terminal prompt")).toBeVisible();
  await expect(page.getByLabel("Terminal prompt")).toHaveValue(
    "keep this draft",
  );
});
