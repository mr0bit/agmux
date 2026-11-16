import { expect, test } from '@playwright/test';
test('hosted design preview explains the actual backend launch and validates local companion URLs', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Connect backend', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Connect backend' })).toBeVisible();
  await expect(page.getByRole('dialog')).toContainText('npm run live');
  await page.getByLabel('Companion URL').fill('javascript:alert(1)');
  await expect(page.getByRole('link', { name: 'Open live companion' })).toHaveCount(0);
  await page.getByLabel('Companion URL').fill('http://127.0.0.1:4242/');
  await expect(page.getByRole('link', { name: 'Open live companion' })).toHaveAttribute('href','http://127.0.0.1:4242/');
});
