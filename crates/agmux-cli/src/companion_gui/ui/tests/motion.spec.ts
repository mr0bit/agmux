import { expect, test } from "@playwright/test";

test("disclosures, panels and dialogs settle into usable states", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/");
  const tool = page.getByLabel("Inspect workspace");
  await tool.getByRole("button").click();
  await expect(tool.getByText("cat companion_gui.rs")).toBeVisible();
  await tool.getByRole("button").click();
  await expect(tool.locator(".tool-expander")).toHaveCount(0);
  await page.getByRole("button", { name: "Close review", exact: true }).click();
  await expect(page.locator(".review-dock")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Toggle review panel", exact: true })
    .click();
  await expect
    .poll(() =>
      page
        .locator(".review-dock")
        .evaluate((element) =>
          Math.round(element.getBoundingClientRect().width),
        ),
    )
    .toBe(427);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("reduced motion leaves controls immediately usable without perpetual animation", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "New session" })).toBeVisible();
  await page.getByRole("button", { name: "Close dialog", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      document
        .getAnimations()
        .every(
          (animation) =>
            animation.playState !== "running" ||
            animation.effect?.getTiming().duration === 0,
        ),
    ),
  ).toBeTruthy();
});

test("streaming does not restart message entrance or pull the reader to the bottom", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/");
  const input = page.getByLabel("Message agent");
  await input.fill("Check the motion while I read back.");
  await input.press("Enter");
  await expect(
    page.locator(".user-message").filter({ hasText: "Check the motion" }),
  ).toBeVisible();
  await page.waitForTimeout(400);
  const conversation = page.getByLabel("Conversation");
  await conversation.evaluate((element) => {
    element.scrollTop = 0;
  });
  await page.waitForTimeout(500);
  expect(
    await conversation.evaluate((element) => element.scrollTop),
  ).toBeLessThan(5);
  await expect(
    page.getByRole("button", { name: /new message|Jump to latest/ }),
  ).toBeVisible();
});
