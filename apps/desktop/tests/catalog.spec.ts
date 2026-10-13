import { test, expect } from '@playwright/test';

test('catalog preview filters, inspects 3D items, and keeps actions simulated', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/test/project';
      if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Catalog preview' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['learning_set_list', 'simulation_list', 'builder_list'].includes(command)) return [];
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Catalog', exact: true }).click();
  const workspace = page.getByRole('region', { name: 'Catalog workspace' });
  await expect(workspace).toContainText('6 items');
  await expect(workspace.getByRole('img', { name: 'Compact robot arm' })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-catalog.png' });
  await page.getByRole('navigation', { name: 'Catalog categories' }).getByRole('button', { name: 'Components' }).click();
  await expect(workspace).toContainText('2 items');
  await page.getByRole('textbox', { name: 'Search catalog' }).fill('camera');
  await expect(workspace).toContainText('1 item');
  await workspace.getByRole('button', { name: 'View Depth camera 1.0.0' }).click();
  await expect(workspace.getByRole('heading', { name: 'Depth camera', exact: true })).toBeVisible();
  await expect(workspace.locator('canvas')).toBeVisible();
  await workspace.getByRole('button', { name: 'Download 1.0.0' }).click();
  await expect(workspace.getByRole('button', { name: 'Add to project' })).toBeVisible();
  await workspace.getByRole('button', { name: 'Add to project' }).click();
  await expect(page.getByText('Builder integration is coming next.', { exact: false })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-catalog-detail.png' });
  await workspace.getByRole('button', { name: 'All items', exact: true }).click();
  await page.getByRole('textbox', { name: 'Search catalog' }).fill('not-a-part');
  await expect(workspace.getByRole('heading', { name: 'No matching items' })).toBeVisible();
  await workspace.getByRole('button', { name: 'Clear filters' }).click();
  await page.getByLabel('Catalog availability').selectOption('offline');
  await expect(workspace).toContainText('2 items');
  await expect(workspace.getByRole('button', { name: 'Connect cloud' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Catalog connection settings' }).click();
  await expect(page.getByRole('dialog')).toContainText('Coming soon');
  await page.getByRole('button', { name: 'Got it' }).click();
  expect(errors).toEqual([]);
});
