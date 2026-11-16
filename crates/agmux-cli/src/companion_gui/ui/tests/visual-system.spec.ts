import { expect, test } from "@playwright/test";

test("themes and density persist, and focus mode restores the workspace", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Light", exact: true }).click();
  await page.getByRole("button", { name: "Compact", exact: true }).click();
  await expect(page.locator(".app")).toHaveAttribute("data-theme", "light");
  await expect(page.locator(".app")).toHaveAttribute("data-density", "compact");
  expect(
    await page
      .locator(".app")
      .evaluate((element) => getComputedStyle(element).filter),
  ).toBe("none");
  await page.getByRole("button", { name: "Done", exact: true }).click();
  expect(
    await page
      .locator(".session-item")
      .first()
      .evaluate((element) => element.clientHeight),
  ).toBeLessThan(50);
  await page.reload();
  await expect(page.locator(".app")).toHaveAttribute("data-theme", "light");
  await expect(page.locator(".app")).toHaveAttribute("data-density", "compact");
  await page.getByRole("button", { name: "Enter focus view" }).click();
  await expect(page.locator(".sidebar")).toHaveCount(0);
  await expect(page.locator(".review-pane")).toHaveCount(0);
  await expect(page.getByLabel("Message agent")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".sidebar")).toBeVisible();
  await expect(page.locator(".review-pane")).toBeVisible();
});

test("timeline activity collapses and both diff layouts and CodeMirror work", async ({
  page,
}) => {
  await page.goto("/");
  const tool = page.getByLabel("Inspect workspace");
  await expect(tool.getByRole("button")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await tool.getByRole("button").click();
  await expect(tool.getByText("cat companion_gui.rs")).toBeVisible();
  await page
    .locator(".review-toolbar")
    .getByRole("button", { name: "Side by side", exact: true })
    .click();
  await expect(page.getByLabel("Side-by-side diff")).toBeVisible();
  await expect(page.locator(".split-cell.del")).toHaveCount(4);
  await page.getByRole("button", { name: "Files", exact: true }).click();
  const editor = page.getByLabel("Code editor");
  await expect(editor).toContainText("SessionList");
  await editor.fill("export const answer = 42;");
  await editor.press("Control+s");
  await expect(page.getByRole("status")).toContainText("Saved to this browser");
  await page.reload();
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await expect(page.getByLabel("Code editor")).toContainText(
    "export const answer = 42;",
  );
});

test("empty search, machine offline, completed review and fresh sessions are actionable", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Search sessions").fill("not-a-session");
  await expect(
    page.getByRole("heading", { name: "Nothing here just yet" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Clear filters", exact: true })
    .click();
  await expect(page.locator(".session-item").first()).toBeVisible();
  await page.getByRole("button", { name: "Machines 1", exact: true }).click();
  await page.getByRole("button", { name: "Preview offline state" }).click();
  await expect(
    page.getByRole("heading", { name: "This machine is taking a break" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Back to machine preview" }).click();
  await expect(page.locator(".machine-preview-card")).toBeVisible();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page
    .locator(".review-bottom")
    .getByRole("button", { name: "Mark reviewed" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Review complete" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Reopen review", exact: true })
    .click();
  await expect(page.locator(".review-scroll")).toBeVisible();
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await page.getByLabel("Session name").fill("A fresh idea");
  await page
    .getByRole("button", { name: "Create session", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "What are we building?" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Plan a new feature", exact: true })
    .click();
  await expect(page.getByLabel("Message agent")).toHaveValue(
    "Plan a new feature",
  );
});

test("an empty workspace can create its first session", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("agmux-demo-sessions", "[]"),
  );
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Your next idea starts here" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Start your first session" }).click();
  await page.getByLabel("Session name").fill("First project");
  await page
    .getByRole("button", { name: "Create session", exact: true })
    .click();
  await expect(page.locator("h1")).toHaveText("First project");
  await expect(page.getByLabel("Message agent")).toBeVisible();
});
