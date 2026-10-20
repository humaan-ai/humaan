import { test, expect } from '@playwright/test';

test('overview previews project URDFs before any Builder version is saved', async ({ page }) => {
  await page.addInitScript(() => {
    const source = 'robots/SO101/camera.urdf';
    const xml = '<robot name="Camera arm"><link name="base"><visual><geometry><box size=".3 .2 .4"/></geometry></visual></link></robot>';
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string, args: any) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/workspace/pick-and-place';
      if (command === 'experience_project') return { root: '/workspace/pick-and-place', project: { id: 'urdf-project', name: 'Pick and place' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['learning_set_list', 'builder_list', 'simulation_list', 'deployment_list'].includes(command)) return [];
      if (command === 'builder_project') return {
        activeRobotId: 'camera',
        robots: [
          { id: 'arm', name: 'Base arm', source: 'robots/SO101/arm.urdf', missing: false },
          { id: 'camera', name: 'Camera arm', source, missing: false },
        ],
      };
      if (command === 'builder_draft_load') return null;
      if (command === 'builder_robot_files') return [{ path: source, size: xml.length }];
      if (command === 'builder_read_robot_file') {
        if (args.path !== source) throw new Error('Unexpected URDF preview.');
        return Array.from(new TextEncoder().encode(xml));
      }
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  const current = page.getByRole('region', { name: 'Current robot', exact: true });
  await expect(current.getByRole('heading', { name: 'Camera arm', exact: true })).toBeVisible();
  await expect(current.getByRole('img', { name: '3D preview of Camera arm', exact: true })).toBeVisible();
  await expect(current).toContainText('robots/SO101/camera.urdf');
  await expect(current).toContainText('2 robots in this project.');
  await expect(page.getByRole('button', { name: '0 Robot versions', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Give your robot a starting point' })).toHaveCount(0);
});

test('overview shows real saved activity, refresh failures, and workflow navigation', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const pose = { xyz: [0,0,0], rpy: [0,0,0] };
    const robot = { schema: 1, name: 'Inspection arm', joints: [], assets: {}, links: [{ id: 'base', name: 'base', visuals: [
      { kind: 'box', size: [.25,.25,.06], origin: pose, color: '#788795' },
      { kind: 'cylinder', size: [.065,.35,1], origin: { xyz: [0,0,.2], rpy: [0,0,0] }, color: '#7897b5' },
      { kind: 'box', size: [.3,.07,.07], origin: { xyz: [.12,0,.4], rpy: [0,.3,0] }, color: '#7897b5' }
    ], collisions: [], mass: 1, inertia: [1,0,0,1,0,1], inertial: pose, sensors: [], mounts: [] }] };
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/workspace/inspection-arm';
      if (command === 'experience_project') return { root: '/workspace/inspection-arm', project: { id: 'project-123', name: 'Inspection arm' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (command === 'learning_set_list') return [];
      if (command === 'builder_project') return { activeRobotId: null, robots: [] };
      if (command === 'builder_draft_load') return null;
      if (['builder_activate_robot', 'builder_draft_save'].includes(command)) return;
      if (command === 'builder_list') return [{ id: '1-2', name: 'Inspection arm · v2', links: 6, savedAt: 1780000000000 }];
      if (command === 'builder_load') return robot;
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      if (command === 'simulation_list') {
        if ((window as any).__failOverview) throw new Error('Run index unavailable');
        return [{ id: 'run1', name: 'Joint range check', createdAt: 1780000300000, status: 'completed', versionId: '1-2' }];
      }
      if (command === 'deployment_list') return [{ id: 'session1', name: 'Calibration check', createdAt: 1780000600000 }];
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/'); await page.getByRole('button', { name: 'Open project', exact: true }).click();
  const overview = page.getByRole('region', { name: 'Project workspace' });
  await expect(overview.getByRole('heading', { name: 'Inspection arm', exact: true })).toBeVisible();
  await expect(overview.getByRole('img', { name: '3D preview of Inspection arm · v2' })).toBeVisible();
  await expect(overview.getByRole('button', { name: '1 Robot versions' })).toBeVisible();
  await expect(overview.getByRole('button', { name: '1 Simulation runs' })).toBeVisible();
  await expect(overview.getByText('Calibration check', { exact: true })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-overview-populated.png' });
  await page.setViewportSize({ width: 960, height: 800 });
  await expect(overview.getByRole('button', { name: 'Continue in Builder' })).toBeVisible();
  expect(await overview.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await page.screenshot({ path: '/tmp/humaan-overview-compact.png' });
  await page.evaluate(() => { (window as any).__failOverview = true; });
  await overview.getByRole('button', { name: 'Refresh project overview' }).click();
  await expect(overview.getByText('Some project information is unavailable. Refresh to retry.')).toBeVisible();
  await expect(overview.getByRole('button', { name: '— Simulation runs' })).toBeVisible();
  await overview.getByRole('button', { name: 'Continue in Builder' }).click();
  await expect(page.getByRole('button', { name: 'Builder', exact: true })).toHaveAttribute('aria-current', 'page');
  expect(errors).toEqual([]);
});
