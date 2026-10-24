import { test, expect } from './fixtures';

test('catalog is empty without sample items or simulated actions', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/test/project';
      if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Catalog preview' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['learning_set_list', 'simulation_list', 'builder_list'].includes(command)) return [];
      if (command === 'builder_project') return { activeRobotId: null, robots: [] };
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Catalog', exact: true }).click();
  const workspace = page.getByRole('region', { name: 'Catalog workspace' });
  await expect(workspace).toContainText('0 items');
  await expect(workspace.getByRole('heading', { name: 'No catalog items yet' })).toBeVisible();
  await expect(workspace.getByRole('button')).toHaveCount(0);
  await expect(workspace.locator('canvas')).toHaveCount(0);
  await page.getByRole('navigation', { name: 'Catalog categories' }).getByRole('button', { name: 'Components' }).click();
  await expect(workspace.getByRole('heading', { name: 'No catalog items yet' })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-catalog-empty.png' });
  expect(errors).toEqual([]);
});
