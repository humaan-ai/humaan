import { test, expect } from '@playwright/test';

test('robot connection, live mapping, recording recovery and session replay', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    const pose = { xyz: [0, 0, 0], rpy: [0, 0, 0] };
    const link = (id: string) => ({ id, name: id, visuals: [{ kind: 'box', size: [.1,.1,.3], asset: '', origin: pose, color: '#8ca8c3' }], collisions: [], mass: 1, inertia: [1,0,0,1,0,1], inertial: pose, mounts: [], sensors: [] });
    const robot = { schema: 1, name: 'SO-101', links: [link('base'), link('arm')], assets: {}, joints: [{ id: 'joint', name: 'shoulder', type: 'revolute', parent: 'base', child: 'arm', axis: [0,1,0], origin: pose, lower: -3.14, upper: 3.14, effort: 10, velocity: 1, hardware: '' }] };
    const data: Record<string, any[]> = { robots: [], sessions: [] }; let sequence = 0, failSave = true;
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string, args: any) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/test/project';
      if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Live Link' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['learning_set_list', 'simulation_list'].includes(command)) return [];
      if (command === 'builder_list') return [{ id: '1-2', name: 'SO-101', savedAt: 1, links: 2 }];
      if (command === 'builder_load') return robot;
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      if (command === 'deployment_list') return data[args.kind];
      if (command === 'deployment_load') return data[args.kind].find(item => item.id === args.id);
      if (command === 'deployment_save') {
        if (args.kind === 'sessions' && failSave) { failSave = false; throw new Error('Disk unavailable'); }
        const item = { ...args.value, id: crypto.randomUUID(), ...(args.kind === 'robots' ? { robot } : {}) }; data[args.kind].push(item); return item;
      }
      if (command === 'deployment_sample') return { sequence: (window as any).__freeze ? sequence : ++sequence, positions: { shoulder: 0.2 } };
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/'); await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Deployments', exact: true }).click();
  const workspace = page.getByRole('region', { name: 'Deployments workspace' });
  await workspace.getByRole('button', { name: 'Add robot', exact: true }).click();
  await page.getByLabel('Builder version', { exact: true }).selectOption('1-2');
  await page.getByRole('button', { name: 'Save robot', exact: true }).click();
  await workspace.getByRole('button', { name: 'Live Link', exact: true }).click();
  await expect(workspace.getByRole('status')).toHaveText('Live · Read only');
  await expect(workspace.getByText('0.200 rad', { exact: true })).toBeVisible();
  await workspace.getByRole('button', { name: 'Record session', exact: true }).click();
  await expect(workspace.getByRole('button', { name: /Stop & save · [1-9]/ })).toBeVisible();
  await workspace.getByRole('button', { name: /Stop & save/ }).click();
  await expect(workspace.getByRole('alert')).toContainText('Disk unavailable');
  await workspace.getByRole('button', { name: 'Retry save' }).click();
  await expect(workspace.getByRole('button', { name: 'Retry save' })).toHaveCount(0);
  await workspace.getByRole('button', { name: 'Disconnect', exact: true }).click();
  await page.getByRole('navigation', { name: 'Deployment sections' }).getByRole('button', { name: 'Sessions' }).click();
  await page.getByRole('complementary', { name: 'Deployments', exact: true }).getByRole('button', { name: /SO-101 ·/ }).click();
  await expect(workspace.getByLabel('Session timeline')).toBeVisible();
  await expect(workspace.getByText('1-2', { exact: true })).toBeVisible();
  expect((await workspace.locator('canvas').boundingBox())!.height).toBeGreaterThan(500);
  await page.screenshot({ path: '/tmp/humaan-deployments-session.png' });
  await page.getByRole('navigation', { name: 'Deployment sections' }).getByRole('button', { name: 'Robots', exact: true }).click();
  await page.getByRole('complementary', { name: 'Deployments', exact: true }).getByRole('button', { name: 'SO-101', exact: true }).click();
  await workspace.getByRole('button', { name: 'Live Link', exact: true }).click();
  await expect(workspace.getByRole('status')).toHaveText('Live · Read only');
  await workspace.getByRole('button', { name: 'Record session', exact: true }).click();
  await expect(workspace.getByRole('button', { name: /Stop & save · [1-9]/ })).toBeVisible();
  await page.evaluate(() => { (window as any).__freeze = true; });
  await expect(workspace.getByRole('status')).toHaveText('Disconnected');
  await expect(workspace.getByRole('alert')).toContainText('Telemetry is stale');
  expect(errors).toEqual([]);
});
