import { expect, test } from "@playwright/test";

test("panel divider and visibility survive reload; session shortcuts and recent history", async ({
  page,
}) => {
  await page.goto("/");
  const divider = page.getByRole("separator", { name: "Resize review panel" });
  await divider.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(divider).toHaveAttribute("aria-valuenow", "444");
  await page.reload();
  await expect(divider).toHaveAttribute("aria-valuenow", "444");
  await page.getByRole("button", { name: "Close review", exact: true }).click();
  await page.reload();
  await expect(divider).toHaveCount(0);
  await page
    .getByRole("button", { name: "Toggle review panel", exact: true })
    .click();
  await expect(divider).toBeVisible();
  await page.keyboard.press("Alt+ArrowDown");
  await expect(page.locator("h1")).toHaveText("Review PTY lifecycle");
  await page
    .getByRole("button", { name: "Recent sessions", exact: true })
    .click();
  await page
    .locator(".recent-popover")
    .getByRole("button", { name: /Build the companion/ })
    .click();
  await expect(page.locator("h1")).toHaveText("Build the companion workspace");
  await page.keyboard.press("Alt+a");
  await page.getByLabel("Search agents").fill("gem");
  await page
    .locator(".agent-popover")
    .getByRole("button", { name: "Gemini", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Quick agent picker" }),
  ).toContainText("Gemini");
});

test("composer grows, mentions are searchable, and attachments have previews", async ({
  page,
}) => {
  await page.goto("/");
  const input = page.getByLabel("Message agent");
  const initial = await input.evaluate((element) => element.clientHeight);
  await input.fill("Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6");
  expect(
    await input.evaluate((element) => element.clientHeight),
  ).toBeGreaterThan(initial);
  await input.fill("@cod");
  await expect(
    page.getByRole("option", { name: /Codex/ }).first(),
  ).toBeVisible();
  await input.press("Enter");
  await expect(input).toHaveValue("@Codex ");
  await page.getByLabel("Upload attachments").setInputFiles({
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("The next step is the Rust bridge."),
  });
  await expect(page.getByLabel("Attachment previews")).toContainText(
    "notes.txt",
  );
  await expect(page.getByLabel("Attachment previews")).toContainText(
    "The next step",
  );
  await page
    .getByRole("button", { name: "Remove notes.txt", exact: true })
    .click();
  await expect(page.getByLabel("Attachment previews")).toHaveCount(0);
});

test("queue can reorder, pause, survive reload, resume and remain session-local", async ({
  page,
}) => {
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 100));
  await page.goto("/");
  const input = page.getByLabel("Message agent");
  await input.fill("First task");
  await input.press("Enter");
  await input.fill("Next task");
  await input.press("Enter");
  await input.fill("Last task");
  await input.press("Enter");
  await expect(page.getByLabel("Queued messages")).toContainText("Next task");
  await page.getByRole("button", { name: "Move queued message 2 up" }).click();
  await expect(page.locator(".queue-text").first()).toContainText("Last task");
  await page
    .getByRole("button", { name: "Stop response", exact: true })
    .click();
  await expect(page.getByLabel("Preview session status")).toContainText(
    "Stopped",
  );
  await page.clock.resume();
  await page.reload();
  await expect(page.getByLabel("Queued messages")).toContainText("Last task");
  await page.keyboard.press("Alt+ArrowDown");
  await expect(page.getByLabel("Queued messages")).toHaveCount(0);
  await page.keyboard.press("Alt+ArrowUp");
  await expect(page.getByLabel("Queued messages")).toContainText("Next task");
  await page.getByRole("button", { name: "Resume queue", exact: true }).click();
  await expect(
    page.locator(".user-message").filter({ hasText: "Last task" }),
  ).toBeVisible();
});

test("reconnecting protects drafts and long histories mount bounded rows without losing scroll", async ({
  page,
}) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      "agmux-demo-messages",
      JSON.stringify({
        workspace: Array.from({ length: 600 }, (_, index) => ({
          id: `message-${index}`,
          role: index % 2 ? "assistant" : "user",
          text: `Message ${index} ` + "Long history content. ".repeat(15),
        })),
      }),
    );
    localStorage.setItem(
      "agmux-demo-sessions",
      JSON.stringify(
        Array.from({ length: 600 }, (_, index) => ({
          id: index ? `session-${index}` : "workspace",
          title: index ? `Session ${index}` : "Build the companion workspace",
          state: "idle",
          agent: "Codex",
          project: "agmux",
          time: "now",
        })),
      ),
    );
  });
  await page.goto("/");
  expect(await page.locator(".session-item").count()).toBeLessThan(30);
  expect(await page.locator(".transcript-row").count()).toBeLessThan(25);
  const conversation = page.getByLabel("Conversation");
  await conversation.evaluate((element) => {
    element.scrollTop = 2500;
  });
  await expect
    .poll(() => conversation.evaluate((element) => element.scrollTop))
    .toBeGreaterThan(1000);
  const before = await conversation.evaluate((element) => element.scrollTop);
  await page.keyboard.press("Alt+ArrowDown");
  await page.keyboard.press("Alt+ArrowUp");
  await expect
    .poll(() =>
      conversation.evaluate((element, offset) => Math.abs(element.scrollTop - offset), before),
    )
    .toBeLessThan(300);
  await page.getByLabel("Message agent").fill("Keep this draft");
  await page.getByLabel("Preview session status").click();
  await page
    .locator(".status-popover")
    .getByRole("button", { name: "Reconnecting", exact: true })
    .click();
  await expect(page.getByLabel("Message agent")).toBeDisabled();
  await expect(page.getByLabel("Message agent")).toHaveValue("Keep this draft");
  await page.getByLabel("Preview session status").click();
  await page
    .locator(".status-popover")
    .getByRole("button", { name: "Ready", exact: true })
    .click();
  await expect(page.getByLabel("Message agent")).toBeEnabled();
});
