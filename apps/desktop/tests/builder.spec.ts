import { test, expect, type Page } from '@playwright/test';

async function openBuilder(page: Page) {
  await page.addInitScript(() => {
    // Exercise the real UI with an isolated IPC adapter; disk persistence is tested in Rust.
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (command: string, args: any) => {
        if (command === 'select_directory') return '/test/project';
        if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Builder test' } };
        if (command === 'experience_list') return { records: [], warning: null };
        if (command === 'experience_reviews') return {};
        if (command === 'learning_set_list') return [];
        if (command === 'dataset_tools_status') return { ready: false, custom: true, installing: false, phase: 'Custom dataset environment', error: null };
        if (command === 'simulation_tools_status') return { ready: !!localStorage.getItem('test-mujoco-installed'), installing: false, phase: localStorage.getItem('test-mujoco-installed') ? 'MuJoCo 3.2.7 is ready' : 'MuJoCo is not installed', version: '3.2.7', python: '/managed/python', error: null };
        if (command === 'simulation_tools_install') {
          if (localStorage.getItem('test-mujoco-fail')) throw new Error('Download failed. Check your connection and retry.');
          localStorage.setItem('test-mujoco-installed', 'true');
          return { ready: true, installing: false, phase: 'MuJoCo 3.2.7 is ready', version: '3.2.7', python: '/managed/python', error: null };
        }
        if (command === 'simulation_delete') { localStorage.setItem('test-simulation-runs', JSON.stringify(JSON.parse(localStorage.getItem('test-simulation-runs') || '[]').filter((r: any) => r.id !== args.id))); return; }
        if (command === 'simulation_list') return JSON.parse(localStorage.getItem('test-simulation-runs') || '[]').map(({ robot, result, config, ...summary }: any) => summary);
        if (command === 'simulation_load') return JSON.parse(localStorage.getItem('test-simulation-runs') || '[]').find((r: any) => r.id === args.id);
        if (command === 'simulation_run') {
          localStorage.setItem('test-last-simulation-python', args.python);
          const robot = JSON.parse(localStorage.getItem('test-builder-versions') || '[]').find((s: any) => s.version.id === args.versionId).robot;
          const runs = JSON.parse(localStorage.getItem('test-simulation-runs') || '[]');
          const run = { id: String(runs.length + 1), versionId: args.versionId, name: robot.name, createdAt: Date.now(), status: args.python === 'missing-python' ? 'failed' : 'completed', robot, config: args.config,
            result: args.python === 'missing-python' ? { error: 'Cannot start Python. Select a Python executable with MuJoCo installed.' } : { engine: 'MuJoCo test adapter', maxContacts: 0, warnings: ['Fixed root at the origin.'], frames: [0, .05, .1].map(time => ({ time, poses: Object.fromEntries(robot.links.map((l: any) => [l.id, { position: [0, 0, time], quaternion: [0, 0, 0, 1] }])), joints: {} })) } };
          runs.push(run); localStorage.setItem('test-simulation-runs', JSON.stringify(runs)); return run;
        }
        if (command === 'builder_delete_version') {
          const key = 'test-builder-versions';
          localStorage.setItem(key, JSON.stringify(JSON.parse(localStorage.getItem(key) || '[]').filter((snapshot: any) => snapshot.version.id !== args.id))); return;
        }
        if (command === 'builder_list' || command === 'builder_save' || command === 'builder_load') {
          const key = 'test-builder-' + args.kind;
          const snapshots = JSON.parse(localStorage.getItem(key) || '[]');
          if (command === 'builder_list') return snapshots.map((s: any) => s.version).reverse();
          if (command === 'builder_load') return snapshots.find((s: any) => s.version.id === args.id).robot;
          const version = { id: String(snapshots.length + 1), name: args.robot.name, savedAt: Date.now(), links: args.robot.links.length };
          snapshots.push({ version, robot: args.robot }); localStorage.setItem(key, JSON.stringify(snapshots)); return version;
        }
        if (command.startsWith('plugin:event|')) return 1;
        throw new Error('Unexpected IPC: ' + command);
      },
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Builder', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Import URDF', exact: true })).toBeEnabled();
}

test('assemble, configure, reuse, and reopen immutable versions', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await openBuilder(page);
  await page.locator('summary').filter({ hasText: 'Add to assembly' }).click();
  await expect(page.getByRole('button', { name: 'Add module', exact: true })).toBeDisabled();
  await page.locator('summary').filter({ hasText: /^Parts/ }).click();
  await expect(page.getByRole('region', { name: 'Parts catalog' }).getByRole('button')).toHaveCount(0);
  await expect(page.getByText('Select a part from the catalog to preview it.')).toBeVisible();
  const xml = '<robot name="Fixture"><link name="base"><visual><geometry><box size="0.4 0.3 0.15"/></geometry></visual></link></robot>';
  await page.locator('input[type=file]').first().setInputFiles([{ name: 'fixture.urdf', mimeType: 'application/xml', buffer: Buffer.from(xml) }]);
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await page.getByRole('button', { name: /^Rename robot / }).dblclick();
  await page.getByRole('textbox', { name: 'Robot name', exact: true }).fill('Inspection robot');
  await page.getByRole('spinbutton', { name: 'Mass (kg)', exact: true }).fill('4.5');
  await page.getByRole('spinbutton', { name: 'Mount position (m) Z', exact: true }).fill('0.2');
  await page.getByRole('combobox', { name: 'Module', exact: true }).selectOption('link:base');
  await page.getByRole('button', { name: 'Add module', exact: true }).click();
  await page.getByRole('combobox', { name: 'Type', exact: true }).selectOption('revolute');
  await page.getByRole('textbox', { name: 'Hardware mapping', exact: true }).fill('can0/motor_1');
  await page.locator('summary').filter({ hasText: /^Parts/ }).click();
  await expect(page.getByRole('region', { name: 'Assembly', exact: true })).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'Part information' })).toContainText('Total mass');
  await page.getByRole('textbox', { name: 'Search Builder' }).fill('base_2');
  await expect(page.getByRole('region', { name: 'Assembly', exact: true }).locator('button[title]')).toHaveCount(2);
  await expect(page.getByRole('region', { name: 'Parts catalog' }).getByRole('button')).toHaveCount(1);
  await page.getByRole('region', { name: 'Parts catalog' }).getByRole('button').click();
  await expect(page.getByRole('complementary', { name: 'Part information' })).toContainText('0.4 × 0.3 × 0.15 m');
  await page.getByRole('textbox', { name: 'Search Builder' }).fill('');
  await page.getByRole('button', { name: 'Builder filters', exact: true }).click();
  await expect(page.getByRole('menu')).toBeVisible();
  await page.getByRole('menuitemradio', { name: 'Robot parts', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Parts catalog' }).getByRole('button')).toHaveCount(2);
  await page.getByRole('button', { name: 'View all parts', exact: true }).click();
  const catalog = page.getByRole('region', { name: 'All parts catalog', exact: true });
  await expect(catalog.getByRole('button', { name: /^Preview / })).toHaveCount(2);
  await expect(catalog.getByRole('img', { name: '3D preview of base', exact: true })).toBeVisible();
  await expect(catalog.getByRole('img', { name: '3D preview of base_2', exact: true })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Builder modes' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Visual', exact: true })).toHaveCount(0);
  await page.screenshot({ path: '/tmp/humaan-parts-catalog.png' });

  await page.getByRole('textbox', { name: 'Search Builder' }).fill('base_2');
  await expect(catalog.getByRole('button', { name: /^Preview / })).toHaveCount(1);
  await catalog.getByRole('button', { name: 'Preview base_2', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Part information' })).toContainText('base_2');
  await page.getByRole('button', { name: 'Back to catalog' }).click();
  await page.getByRole('textbox', { name: 'Search Builder' }).fill('');
  await expect(catalog.getByRole('button', { name: /^Preview / })).toHaveCount(2);
  await page.getByRole('region', { name: 'Assembly', exact: true }).locator('button[title]').last().click();

  const assembly = page.getByRole('region', { name: 'Assembly', exact: true });
  await page.getByRole('button', { name: 'Builder filters', exact: true }).click();
  await page.getByRole('menuitemradio', { name: 'Prismatic', exact: true }).click();
  await expect(assembly).toContainText('No matching assembly parts.');
  await page.getByRole('button', { name: 'Builder filters', exact: true }).click();
  await page.getByRole('menuitemradio', { name: 'All joint types', exact: true }).click();
  await page.getByRole('button', { name: 'Builder filters', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menu')).toHaveCount(0);
  await expect(assembly.locator('button[title]')).toHaveCount(2);
  await page.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await expect(assembly.locator('button[title]')).toHaveCount(1);
  await assembly.getByRole('button', { name: /^Expand / }).click();
  await expect(assembly.locator('button[title]')).toHaveCount(2);
  await assembly.locator('button[title]').first().press('ArrowLeft');
  await expect(assembly.locator('button[title]')).toHaveCount(1);
  await page.getByRole('button', { name: 'Expand all', exact: true }).click();
  await expect(assembly.locator('button[title]')).toHaveCount(2);

  await page.getByRole('button', { name: 'Save version' }).click();
  await expect(page.getByText('Saved Inspection robot.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save assembly as module' }).click();
  await expect(page.getByText('Added Inspection robot to the module library.')).toBeVisible();
  await page.getByRole('button', { name: 'Test', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Assembly checks' })).toContainText('inertia tensor');
  await expect(page.getByRole('slider')).toHaveCount(1);
  await page.getByRole('button', { name: 'Show bottom panel', exact: true }).click();
  const menu = await page.getByRole('complementary', { name: 'Builder', exact: true }).boundingBox();
  const terminal = await page.getByRole('region', { name: 'Terminal', exact: true }).boundingBox();
  expect(terminal!.x).toBeGreaterThanOrEqual(menu!.x + menu!.width);
  await page.screenshot({ path: '/tmp/humaan-builder.png' });
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await page.getByRole('combobox', { name: 'Module', exact: true }).selectOption('1');
  await page.getByRole('button', { name: 'Add module', exact: true }).click();
  await expect(page.getByRole('slider')).toHaveCount(1);
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.locator('summary').filter({ hasText: 'Saved versions' }).click();
  const versionList = page.getByRole('group', { name: 'Saved versions', exact: true });
  await versionList.getByRole('button', { name: /^Inspection robot/ }).click();
  await expect(versionList.getByRole('button', { name: /^Inspection robot/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('button', { name: 'Rename robot Inspection robot', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Configure', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Mass (kg)', exact: true })).toHaveValue('4.5');
  const snapshots = await page.evaluate(() => JSON.parse(localStorage.getItem('test-builder-versions')!));
  // Reusing the same part aligns its identical parent and child mounting offsets.
  expect(snapshots[0].robot.joints[0].origin.xyz[2]).toBeCloseTo(0);
  expect(snapshots[0].robot.joints[0].hardware).toBe('can0/motor_1');
  await versionList.hover();
  await page.getByRole('button', { name: 'Delete version Inspection robot', exact: true }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(versionList.getByRole('button', { name: /^Inspection robot/ })).toBeVisible();
  await page.getByRole('button', { name: 'Delete version Inspection robot', exact: true }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(versionList).toContainText('No saved versions yet.');
  await expect(page.locator('[data-sonner-toast]').filter({ hasText: 'Deleted saved version Inspection robot.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Rename robot Inspection robot', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save version' })).toBeEnabled();

  expect(errors).toEqual([]);
});

test('imports URDF and mesh assets, rejects missing assets without losing the draft', async ({ page }) => {
  await openBuilder(page);
  const xml = '<robot name="Mesh robot"><link name="base"><visual><geometry><mesh filename="mesh.stl"/></geometry></visual><collision><geometry><box size="1 1 1"/></geometry></collision><inertial><mass value="2"/><inertia ixx="1" ixy="0" ixz="0" iyy="1" iyz="0" izz="1"/></inertial></link></robot>';
  const mesh = 'solid part\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid part';
  await page.locator('input[type=file]').first().setInputFiles([{ name: 'robot.urdf', mimeType: 'application/xml', buffer: Buffer.from(xml) }, { name: 'mesh.stl', mimeType: 'model/stl', buffer: Buffer.from(mesh) }]);
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Rename robot Mesh robot', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save version' }).click();
  await expect(page.getByText('Saved Mesh robot.', { exact: true })).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Builder', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Rename robot Mesh robot', exact: true })).toBeVisible();
  await page.locator('input[type=file]').first().setInputFiles([{ name: 'broken.urdf', mimeType: 'application/xml', buffer: Buffer.from(xml.replace('mesh.stl', 'missing.stl')) }]);
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Missing mesh');
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Rename robot Mesh robot', exact: true })).toBeVisible();
  const snapshots = await page.evaluate(() => JSON.parse(localStorage.getItem('test-builder-versions')!));
  expect(snapshots[0].robot.assets['mesh.stl'].data).toBe(Buffer.from(mesh).toString('base64'));
});

test('exploded inspection keeps saved assembly unchanged', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await openBuilder(page);
  const xml = '<robot name="Explode fixture"><link name="base"><visual><geometry><box size="0.2 0.2 0.1"/></geometry></visual></link><link name="arm"><visual><origin xyz="0 0 0.15"/><geometry><box size="0.08 0.08 0.3"/></geometry></visual></link><joint name="shoulder" type="fixed"><parent link="base"/><child link="arm"/><origin xyz="0 0 0.1"/></joint><link name="tip"/><joint name="tool_mount" type="fixed"><parent link="arm"/><child link="tip"/><origin xyz="0 0 0.3"/></joint></robot>';
  await page.locator('input[type=file]').first().setInputFiles([{ name: 'explode.urdf', mimeType: 'application/xml', buffer: Buffer.from(xml) }]);
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await page.getByRole('button', { name: /^Save version/ }).click();
  const before = await page.evaluate(() => localStorage.getItem('test-builder-versions'));
  await expect(page.getByText('World (0, 0, 0)', { exact: true }).first()).toBeVisible();
  await page.getByRole('button', { name: 'Place on ground', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Restore origin', exact: true })).toBeVisible();
  await page.getByRole('region', { name: 'Assembly', exact: true }).getByRole('button', { name: 'arm', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Restore origin', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Save version/ })).toBeEnabled();
  await page.getByRole('button', { name: 'Restore origin', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Saved', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Explode', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toContainText('arm → base · shoulder · fixed');
  await expect(page.getByRole('button', { name: 'Restore part', exact: true })).toBeDisabled();
  // Fixed fixture and viewport: drag the visible red X-axis handle.
  const drag = async (from: [number, number], to: [number, number]) => {
    await page.mouse.move(...from); await page.mouse.down(); await page.mouse.move(...to, { steps: 12 }); await page.mouse.up();
  };
  await drag([784, 588], [914, 635]);
  await expect(page.getByLabel('Explode connection')).toContainText('1 displaced part');
  await expect(page.getByRole('button', { name: 'Restore part', exact: true })).toBeEnabled();
  await page.screenshot({ path: '/tmp/humaan-explode.png' });
  await drag([914, 635], [784, 588]);
  await expect(page.getByLabel('Explode connection')).toContainText('0 displaced parts');
  await drag([784, 588], [914, 635]);
  await page.getByRole('button', { name: 'Restore part', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toContainText('0 displaced parts');
  await drag([784, 588], [914, 635]);
  await page.getByRole('button', { name: 'Reset all', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toContainText('0 displaced parts');
  await drag([784, 588], [914, 635]);
  await page.getByRole('button', { name: 'Explode', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toHaveCount(0);
  await page.getByRole('button', { name: 'Explode', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toContainText('0 displaced parts');
  await page.getByRole('checkbox', { name: 'Move with children' }).check();
  await drag([784, 588], [914, 635]);
  await expect(page.getByLabel('Explode connection')).toContainText('2 displaced parts');
  await page.getByRole('button', { name: 'Restore part', exact: true }).click();
  await expect(page.getByLabel('Explode connection')).toContainText('0 displaced parts');
  await expect(page.getByRole('button', { name: 'Saved', exact: true })).toBeDisabled();
  expect(await page.evaluate(() => localStorage.getItem('test-builder-versions'))).toBe(before);
  expect(errors).toEqual([]);
});


test('runs simulation from a saved version, replays history, and preserves runtime failures', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await openBuilder(page);
  const xml = '<robot name="Simulation fixture"><link name="base"><visual><geometry><box size="0.1 0.1 0.1"/></geometry></visual><collision><geometry><box size="0.1 0.1 0.1"/></geometry></collision></link></robot>';
  await page.locator('input[type=file]').first().setInputFiles([{ name: 'simulation.urdf', mimeType: 'application/xml', buffer: Buffer.from(xml) }]);
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await page.getByRole('button', { name: 'Test', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open in Simulation' })).toBeDisabled();
  await page.getByRole('button', { name: /^Save version/ }).click();
  await page.getByRole('button', { name: 'Open in Simulation' }).click();
  await expect(page.getByRole('combobox', { name: 'Simulation robot version' })).toHaveValue('1');
  await expect(page.getByText('Ready for local simulation.')).toBeVisible();
  const inspector = page.getByRole('complementary', { name: 'Simulation Inspector' });
  const resize = page.getByRole('separator', { name: 'Resize simulation Inspector' });
  await resize.focus(); await resize.press('ArrowLeft');
  await expect(resize).toHaveAttribute('aria-valuenow', '300');
  await page.getByRole('button', { name: 'Toggle simulation Inspector' }).click();
  await expect(inspector).toBeHidden();
  await expect(page.getByRole('button', { name: 'Simulation status: Ready', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Simulation status: Ready', exact: true }).click();
  await expect(resize).toHaveAttribute('aria-valuenow', '300');
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('Robot version: 1');
  await expect(page.getByRole('button', { name: 'Run details', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Preflight', exact: true }).click();
  await expect(inspector).toContainText('Checks for this run’s saved robot snapshot.');
  await page.getByRole('button', { name: 'Run details', exact: true }).click();
  await page.getByRole('button', { name: 'Play simulation playback' }).click();
  await expect(page.getByRole('slider', { name: 'Simulation timeline' })).toHaveValue('2');
  await page.getByRole('button', { name: 'Reset simulation playback' }).click();
  await expect(page.getByRole('slider', { name: 'Simulation timeline' })).toHaveValue('0');
  await page.screenshot({ path: '/tmp/humaan-simulation.png' });
  await page.getByRole('button', { name: 'Setup', exact: true }).click();
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await page.getByText('Advanced runtime', { exact: true }).click();
  await page.getByRole('textbox', { name: 'Simulation Python executable' }).fill('missing-python');
  await page.getByRole('button', { name: 'Close settings', exact: true }).click();
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('Cannot start Python');
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await page.getByRole('region', { name: 'MuJoCo tools' }).getByRole('button', { name: 'Install MuJoCo', exact: true }).click();
  await expect(page.getByRole('region', { name: 'MuJoCo tools' })).toContainText('MuJoCo 3.2.7 is ready');
  await page.getByRole('button', { name: 'Close settings', exact: true }).click();
  await expect(page.getByLabel('Simulation runtime status')).toContainText('MuJoCo 3.2.7 is ready');
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('Saved run error');
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('Managed MuJoCo is ready now');
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('MuJoCo test adapter');
  expect(await page.evaluate(() => localStorage.getItem('test-last-simulation-python'))).toBe('');
  await page.getByRole('button', { name: 'Close simulation Inspector' }).click();
  await page.reload();
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Simulation', exact: true }).click();
  await page.getByRole('button', { name: /Simulation fixture · completed/ }).first().click();
  await expect(inspector).toBeHidden();
  await page.getByRole('button', { name: 'Toggle simulation Inspector' }).click();
  await expect(resize).toHaveAttribute('aria-valuenow', '300');
  await expect(page.getByRole('complementary', { name: 'Simulation Inspector' })).toContainText('Robot version: 1');
  const savedRuns = page.getByRole('group', { name: 'Saved runs', exact: true });
  await expect(savedRuns.locator('button[aria-pressed="true"]')).toHaveCount(1);
  const versions = await page.evaluate(() => localStorage.getItem('test-builder-versions'));
  await savedRuns.getByRole('button', { name: 'Delete run Simulation fixture', exact: true }).first().click();
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(savedRuns.getByRole('button', { name: 'Delete run Simulation fixture', exact: true })).toHaveCount(3);
  await savedRuns.getByRole('button', { name: 'Delete run Simulation fixture', exact: true }).first().click();
  await page.getByRole('dialog').getByRole('button', { name: 'Delete run', exact: true }).click();
  await expect(savedRuns.getByRole('button', { name: 'Delete run Simulation fixture', exact: true })).toHaveCount(2);
  await expect(page.getByRole('slider', { name: 'Simulation timeline' })).toHaveCount(0);
  await expect(page.locator('[data-sonner-toast]').filter({ hasText: 'Deleted saved run Simulation fixture.' })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('test-builder-versions'))).toBe(versions);
  expect(errors).toEqual([]);
});


test('installs MuJoCo in Settings tools, retries failures, and selects the managed runtime', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await openBuilder(page);
  await page.getByRole('button', { name: 'Simulation', exact: true }).click();
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await page.getByText('Advanced runtime', { exact: true }).click();
  await page.getByRole('textbox', { name: 'Simulation Python executable' }).fill('/custom/python');
  const tools = page.getByRole('region', { name: 'MuJoCo tools' });
  await expect(tools).toContainText('MuJoCo is not installed');
  await page.evaluate(() => localStorage.setItem('test-mujoco-fail', 'true'));
  await tools.getByRole('button', { name: 'Install MuJoCo', exact: true }).click();
  await expect(tools.getByRole('alert')).toContainText('Download failed');
  await page.evaluate(() => localStorage.removeItem('test-mujoco-fail'));
  await tools.getByRole('button', { name: 'Install MuJoCo', exact: true }).click();
  await expect(tools).toContainText('MuJoCo 3.2.7 is ready');
  await expect(tools.getByRole('button', { name: 'Repair MuJoCo' })).toBeEnabled();
  await page.screenshot({ path: '/tmp/humaan-mujoco-tools.png' });
  await expect(page.getByRole('textbox', { name: 'Simulation Python executable' })).toHaveValue('');
  await page.getByRole('button', { name: 'Close settings', exact: true }).click();
  await expect(page.getByText('Advanced runtime', { exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem('humaan-simulation-python'))).toBeNull();
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await expect(tools).toContainText('MuJoCo 3.2.7 is ready');
  expect(errors).toEqual([]);
});


test('migrates legacy system Python default but preserves explicit custom runtime choices', async ({ page }) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem('test-runtime-seeded')) {
      localStorage.setItem('test-runtime-seeded', 'true');
      localStorage.setItem('humaan-simulation-python', 'python3');
      localStorage.setItem('test-mujoco-installed', 'true');
    }
  });
  await openBuilder(page);
  await page.getByRole('button', { name: 'Simulation', exact: true }).click();
  await expect(page.getByLabel('Simulation runtime status')).toContainText('MuJoCo 3.2.7 is ready');
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await page.getByText('Advanced runtime', { exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Simulation Python executable' })).toHaveValue('');
  await page.getByRole('textbox', { name: 'Simulation Python executable' }).fill('python3');
  await page.getByRole('button', { name: 'Close settings', exact: true }).click();
  await page.reload();
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Simulation', exact: true }).click();
  await expect(page.getByLabel('Simulation runtime status')).toContainText('Custom Python runtime');
  await page.getByRole('button', { name: 'Simulation settings' }).click();
  await page.getByText('Advanced runtime', { exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Simulation Python executable' })).toHaveValue('python3');
});
