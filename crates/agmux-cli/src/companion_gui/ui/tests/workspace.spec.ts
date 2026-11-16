import path from "node:path";
import { expect, test } from "@playwright/test";

test("workspace preview and independent session conversations", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Build the companion workspace" }),
  ).toBeVisible();
  await page
    .getByRole("textbox", { name: "Message agent" })
    .fill("Keep this draft");
  await page.getByRole("button", { name: /Review PTY lifecycle/ }).click();
  await expect(
    page.getByRole("textbox", { name: "Message agent" }),
  ).toHaveValue("");
  await page
    .getByRole("button", { name: /Build the companion workspace/ })
    .click();
  await expect(
    page.getByRole("textbox", { name: "Message agent" }),
  ).toHaveValue("Keep this draft");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(
    page.getByText("Keep this draft", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Your message is saved in this demo session.", {
      exact: false,
    }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByText("Keep this draft", { exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("new sessions, agent switching, search, permission, and review", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Allow once", exact: true }).click();
  await expect(
    page.getByText("Permission request resolved in preview"),
  ).toBeVisible();
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await page.getByLabel("Session name").fill("Test Gemini workspace");
  await page.getByRole("button", { name: "Gemini", exact: true }).click();
  await page
    .getByRole("button", { name: "Create session", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Test Gemini workspace", level: 1 }),
  ).toBeVisible();
  await page.getByLabel("Search sessions").fill("Test Gemini");
  await expect(page.locator(".session-item")).toHaveCount(1);
  await page.getByLabel("Search sessions").fill("");
  await page.getByRole("button", { name: "Choose agent", exact: true }).click();
  await page
    .locator(".agent-popover")
    .getByRole("button", { name: "Codex", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Quick agent picker" }),
  ).toContainText("Codex");
  await page.getByRole("button", { name: "Pending", exact: true }).click();
  await expect(page.getByText("No pending changes.")).toBeVisible();
  await page.getByRole("button", { name: "Combined", exact: true }).click();
  await expect(page.locator(".review-scroll .diff-code")).toBeVisible();
});

test("files, terminal, command palette, mobile layout and visual proof", async ({
  page,
}) => {
  await page.goto("/");
  await page.screenshot({
    path: path.resolve("test-results/visual/companion_ui_verification.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await expect(page.getByLabel("Code editor")).toContainText("SessionList");
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.locator(".xterm-screen")).toBeVisible();
  await page.locator(".xterm-helper-textarea").fill("");
  await page.locator(".xterm-helper-textarea").pressSequentially("pwd");
  await page.locator(".xterm-helper-textarea").press("Enter");
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("dialog", { name: "Command palette" }),
  ).toBeVisible();
  await page.getByLabel("Search commands").fill("Open chat");
  await page.getByRole("button", { name: "Open chat" }).click();
  await expect(page.getByLabel("Message agent")).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Hide sidebar", exact: true }).click();
  await expect(page.getByLabel("Message agent")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBeTruthy();
  await page.screenshot({
    path: path.resolve(
      "test-results/visual/companion_ui_mobile_verification.png",
    ),
    fullPage: true,
  });
});
