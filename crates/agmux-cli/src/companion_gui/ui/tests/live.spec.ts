import { expect, test, type Page } from "@playwright/test";
import { readFile, writeFile, symlink, mkdir } from "node:fs/promises";
import path from "node:path";
async function request<T>(
  page: Page,
  route: string,
  body?: unknown,
): Promise<T> {
  return page.evaluate(
    async ({ route, body }) => {
      const response = await fetch(route, {
        method: body === undefined ? "GET" : "POST",
        headers: {
          "X-Agmux-Token": window.__AGMUX__!.token,
          "Content-Type": "application/json",
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      return { status: response.status, value: await response.json() };
    },
    { route, body },
  ) as Promise<T>;
}
async function createShell(page: Page, title: string) {
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await page.getByLabel("Session name").fill(title);
  await page.getByLabel("Agent", { exact: true }).selectOption("shell");
  await page
    .getByRole("button", { name: "Start session", exact: true })
    .click();
  await expect(page.locator("h1")).toHaveText(title);
  await expect(page.locator(".xterm-screen")).toBeVisible();
}
async function send(page: Page, text: string) {
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByLabel("Message agent").fill(text);
  await page.getByRole("button", { name: "Send message", exact: true }).click();
}

test.describe.serial("actual Rust daemon integration", () => {
  test("starts a real shell, sends chat input, survives reload, and streams terminal keystrokes", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "Your next idea starts here" }),
    ).toBeVisible();
    await createShell(page, "Live shell A");
    await send(page, "printf 'UI_'; printf 'BRIDGE_OK\\n'");
    await expect(page.getByLabel("Live session output")).toContainText(
      "UI_BRIDGE_OK",
    );
    await page.reload();
    await expect(page.locator("h1")).toHaveText("Live shell A");
    await expect(page.getByLabel("Live session output")).toContainText(
      "UI_BRIDGE_OK",
    );
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
    await page
      .locator(".xterm-helper-textarea")
      .pressSequentially("printf 'WS_'; printf 'ROUNDTRIP\\n'", { delay: 8 });
    await page.locator(".xterm-helper-textarea").press("Enter");
    await page.getByRole("button", { name: "Chat", exact: true }).click();
    await expect(page.getByLabel("Live session output")).toContainText(
      "WS_ROUNDTRIP",
    );
    expect(errors).toEqual([]);
  });
  test("keeps two sessions isolated and edits real files with actual Git diffs and conflict protection", async ({
    page,
  }) => {
    await page.goto("/");
    await createShell(page, "Live shell B");
    await send(page, "printf 'SECOND_'; printf 'SESSION\\n'");
    await expect(page.getByLabel("Live session output")).toContainText(
      "SECOND_SESSION",
    );
    await expect(page.getByLabel("Live session output")).not.toContainText(
      "UI_BRIDGE_OK",
    );
    await page.getByRole("button", { name: /Live shell A/ }).click();
    await expect(page.getByLabel("Live session output")).toContainText(
      "UI_BRIDGE_OK",
    );
    await expect(page.getByLabel("Live session output")).not.toContainText(
      "SECOND_SESSION",
    );
    await page.getByRole("button", { name: "Files", exact: true }).click();
    await page
      .locator(".file-tree")
      .getByRole("button", { name: "README.md", exact: true })
      .click();
    const editor = page.getByLabel("Code editor");
    await expect(editor).toContainText("# Live integration workspace");
    await editor.fill("# Saved by the live UI\n");
    await page
      .locator(".editor-tab")
      .getByRole("button", { name: "Save", exact: true })
      .click();
    const info = await request<{ status: number; value: { cwd: string } }>(
      page,
      "/api/info",
    );
    expect(await readFile(path.join(info.value.cwd, "README.md"), "utf8")).toBe(
      "# Saved by the live UI\n",
    );
    await page.getByRole("button", { name: "Git", exact: true }).click();
    await expect(page.locator(".git-view .diff-code")).toContainText(
      "Saved by the live UI",
    );
    await page.getByRole("button", { name: "Files", exact: true }).click();
    await editor.fill("# Unsaved browser edit\n");
    await writeFile(
      path.join(info.value.cwd, "README.md"),
      "# External change\n",
    );
    await editor.press("Control+s");
    await expect(page.getByRole("alert").first()).toContainText(
      "File changed on disk",
    );
    expect(await readFile(path.join(info.value.cwd, "README.md"), "utf8")).toBe(
      "# External change\n",
    );
    await expect(editor).toContainText("Unsaved browser edit");
    await page
      .locator(".editor-tab")
      .getByRole("button", { name: "Discard", exact: true })
      .click();
    await page.getByRole("button", { name: "Dismiss error" }).click();
    await page.getByRole("button", { name: "Chat", exact: true }).click();
    await page.screenshot({
      path: path.resolve(
        "../../../../../docs/assets/screenshots/embedded_companion_verification.png",
      ),
      fullPage: true,
    });
  });
  test("enforces authentication, origins, workspace paths, and honest capabilities", async ({
    page,
    request: api,
  }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toHaveText("Live shell A");
    const anonymous = await api.get("/api/snapshot");
    expect(anonymous.status()).toBe(403);
    const key = await page.evaluate(() => window.__AGMUX__!.token);
    const badOrigin = await api.post("/api/reply", {
      headers: { "X-Agmux-Token": key, Origin: "https://evil.invalid" },
      data: { session_id: "anything", message: "blocked" },
    });
    expect(badOrigin.status()).toBe(403);
    const snap = await request<{ status: number; value: { selected: string } }>(
      page,
      "/api/snapshot",
    );
    const traversal = await request<{ status: number }>(page, "/api/files", {
      session_id: snap.value.selected,
      path: "..",
    });
    expect(traversal.status).toBe(403);
    const info = await request<{
      status: number;
      value: { cwd: string; capabilities: { structured_chat: boolean } };
    }>(page, "/api/info");
    expect(info.value.capabilities.structured_chat).toBe(true);
    const outside = path.join(info.value.cwd, "..", "outside");
    await mkdir(outside, { recursive: true });
    await writeFile(path.join(outside, "private.txt"), "must stay outside");
    await symlink(outside, path.join(info.value.cwd, "escape"));
    const outsideRead = await request<{ status: number }>(
      page,
      "/api/panels/open",
      { session_id: snap.value.selected, path: "escape/private.txt" },
    );
    expect(outsideRead.status).toBe(403);
    await expect(
      page.getByRole("button", { name: "Attach files", exact: true }),
    ).toBeDisabled();
  });
  test("delivers an inter-agent message, resizes the real PTY, and retains an ended session snapshot", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /Live shell A/ }).click();
    await page
      .getByRole("button", { name: "Send to session", exact: true })
      .click();
    await page.getByLabel("Recipient").selectOption({ label: "Live shell B" });
    await page
      .getByRole("dialog")
      .getByLabel("Message", { exact: true })
      .fill("printf 'BUS_'; printf 'MESSAGE_OK\\n'");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Send message", exact: true })
      .click();
    await page.getByRole("button", { name: /Live shell B/ }).click();
    await expect(page.getByLabel("Live session output")).toContainText(
      "BUS_MESSAGE_OK",
    );
    await page
      .getByRole("button", { name: "Close review", exact: true })
      .click();
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
    const terminalSize = async () => {
      const state = await request<{ value: { selected: string } }>(
        page,
        "/api/snapshot",
      );
      return page.evaluate(
        (id) =>
          new Promise<number>((resolve, reject) => {
            const socket = new WebSocket(
              `ws://${location.host}/api/terminal?token=${window.__AGMUX__!.token}&session_id=${encodeURIComponent(id)}`,
            );
            const timeout = setTimeout(() => {
              socket.close();
              reject(Error("No terminal sync"));
            }, 5000);
            socket.onmessage = (event) => {
              if (typeof event.data === "string") {
                const value = JSON.parse(event.data);
                if (value.type === "sync") {
                  clearTimeout(timeout);
                  socket.close();
                  resolve(value.cols);
                }
              }
            };
            socket.onerror = () => {
              clearTimeout(timeout);
              reject(Error("Terminal connection failed"));
            };
          }),
        state.value.selected,
      );
    };
    // Wait for the newly opened terminal's first fit, not the prior tab's grid.
    await expect.poll(terminalSize).toBeGreaterThan(110);
    const before = await terminalSize();
    await page.setViewportSize({ width: 1000, height: 800 });
    await expect.poll(terminalSize).toBeLessThan(before);
    await page
      .getByRole("button", { name: "End session", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "End session", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Archived output · this process has ended"),
    ).toBeVisible();
    await expect(page.locator(".live-output")).toContainText("BUS_MESSAGE_OK");
    await page.reload();
    await expect(page.locator("h1")).toHaveText("Live shell B");
    await expect(page.getByLabel("Live session output")).toContainText(
      "BUS_MESSAGE_OK",
    );
    const snapshot = await request<{ value: { selected: string } }>(
      page,
      "/api/snapshot",
    );
    const files = await request<{ status: number }>(page, "/api/files", {
      session_id: snapshot.value.selected,
      path: "",
    });
    expect(files.status).toBe(200);
  });
});

test("Gemini CLI provider setup and real installed CLI launch", async ({ page }) => {
  test.setTimeout(110000);
  await page.goto("/");
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await page.getByLabel("Agent", { exact: true }).selectOption("gemini");
  await expect(page.getByLabel("Gemini model")).toBeVisible();
  await expect(page.getByText("Gemini runs locally.", { exact: false })).toBeVisible();
  const info = await request<{ value: { agents: { id: string; available: boolean }[] } }>(page, "/api/info");
  if (!info.value.agents.find((agent) => agent.id === "gemini")?.available) {
    await expect(page.getByRole("button", { name: "Start session", exact: true })).toBeDisabled();
    await expect(page.getByText("npm install -g @google/gemini-cli", { exact: true })).toBeVisible();
    return;
  }
  // Real provider verification is opt-in; never consume account credentials or API quota.
  test.skip(process.env.AGMUX_GEMINI_TEST !== "1", "Set AGMUX_GEMINI_TEST=1 with the official CLI on PATH");
  await expect(page.getByText("Using CLI configured default",{exact:true})).toBeVisible({timeout:95000});
  await page.getByLabel("Session name").fill("Gemini CLI verification");
  await page.getByText("Advanced settings", {exact:true}).click();
  await page.getByLabel("Arguments (JSON array)").fill('["--version"]');
  await page.getByRole("button", { name: "Start session", exact: true }).click();
  await expect(page.locator("h1")).toHaveText("Gemini CLI verification");
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect.poll(async () => (await request<{value:{screen:string}}>(page,"/api/snapshot")).value.screen).toContain("0.68.0");
  await expect(page.getByRole("log",{name:"Conversation messages"})).not.toContainText("0.68.0");
  await expect(page.locator(".live-chat-header")).toContainText("Gemini");
  await page.screenshot({ path: path.resolve("../../../../../docs/assets/screenshots/gemini_provider_verification.png"), fullPage: true });
});


test("official Gemini ACP reports missing authentication without a stuck turn", async ({page}) => {
  test.skip(process.env.AGMUX_GEMINI_TEST !== "1", "Requires isolated official Gemini CLI");
  test.setTimeout(110000);
  await page.goto("/");
  const info=await request<{value:{cwd:string}}>(page,"/api/info");
  const spawned=await request<{status:number;value:{session_id:string}}>(page,"/api/spawn",{agent:"gemini",cwd:info.value.cwd,title:"Gemini sign-in required",args:[]});
  expect(spawned.status).toBe(200);
  const id=spawned.value.session_id;
  await request(page,"/api/chat",{session_id:id,operation:"send",request_id:"missing-auth",message:"hello"});
  await expect.poll(async () => (await request<{value:{status:string}}>(page,"/api/chat",{session_id:id,operation:"state"})).value.status,{timeout:95000}).toBe("failed");
  const state=await request<{value:{error:string;messages:unknown[]}}>(page,"/api/chat",{session_id:id,operation:"state"});
  expect(state.value.error).toMatch(/auth|API key|sign-in/i);
  expect(state.value.messages).toHaveLength(1);
  await page.reload();await page.getByRole("button",{name:"Chat",exact:true}).click();
  await expect(page.getByLabel("Conversation status")).toHaveText("failed");
  await page.screenshot({path:path.resolve("../../../../../docs/assets/screenshots/qa17_auth_verification.png"),fullPage:true});
});
