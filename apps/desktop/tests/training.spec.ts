import { test, expect } from '@playwright/test';

test('pin a loop experiment, train locally and remotely, compare and reload results', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    const pose = { xyz: [0,0,0], rpy: [0,0,0] };
    const link = (id: string) => ({ id, name: id, mass: 1, inertia: [.01,0,0,.01,0,.01], inertial: pose, visuals: [], collisions: [], mounts: [], sensors: [] });
    const robot = { schema: 1, name: 'Test arm', links: [link('base'),link('arm')], joints: [{ id: 'joint', name: 'joint', type: 'revolute', parent: 'base', child: 'arm', origin: pose, axis: [0,0,1], lower: -1, upper: 1, effort: 10, velocity: 2 }], assets: {} };
    const read = () => JSON.parse(localStorage.getItem('training-catalog') || '{"experiences":[],"loops":[],"environments":[],"tasks":[],"experiments":[],"runs":[]}');
    const save = (c: any) => localStorage.setItem('training-catalog', JSON.stringify(c));
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (command: string, args: any) => {
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'select_directory') return '/test/project';
        if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Training test' } };
        if (command === 'experience_list') return { records: [], warning: null };
        if (command === 'experience_reviews') return {};
        if (['learning_set_list', 'simulation_list', 'deployment_list', 'policy_list'].includes(command)) return [];
        if (command === 'simulation_tools_status') return { ready: true, installing: false, phase: 'Ready', version: '3.2.7' };
        if (command === 'builder_list') return args.kind === 'versions' ? [{ id: '123', name: robot.name, savedAt: 1700000000000, links: 2 }] : [];
        if (command === 'builder_load') return robot;
        if (command === 'training_catalog') return read();
        if (command === 'training_create') {
          const catalog = read(), value = { ...args.input, id: String(Date.now()), createdAt: Date.now() };
          if (args.kind === 'experiments') {
            value.loop = catalog.loops.find((l: any) => l.id === value.loopId);
            value.experience = catalog.experiences.find((e: any) => e.id === value.loop.experienceId);
            value.environment = catalog.environments.find((e: any) => e.id === value.environmentId);
            value.task = catalog.tasks.find((e: any) => e.id === value.taskId);
            value.robot = robot; value.robotName = robot.name;
          }
          catalog[args.kind].push(value); save(catalog); return value;
        }
        if (command === 'training_load') return read()[args.kind].find((e: any) => e.id === args.id);
        if (command === 'training_run') {
          localStorage.setItem('last-training-launch', JSON.stringify(args));
          const catalog = read(), experiment = catalog.experiments.find((e: any) => e.id === args.experimentId);
          const evaluation = { successRate: .8, meanError: .03, meanLoss: .04, seeds: [1000000,1000001,1000002,1000003,1000004] };
          const fail = args.execution.host === 'unreachable';
          const run = { id: String(Date.now()), name: experiment.name, createdAt: Date.now(), status: fail ? 'failed' : 'completed', experiment, experimentId: experiment.id, versionId: '123', experienceId: experiment.experience.id, loopId: experiment.loop.id, robotName: robot.name, execution: args.execution, evaluation, comparisonKey: 'same-protocol', result: fail ? { error: 'SSH connection failed' } : { engine: 'MuJoCo 3.2.7', evaluation, baseline: evaluation, comparisonKey: 'same-protocol', history: [{ iteration: 0, loss: .1 },{ iteration: 1, loss: .04 }], policy: { format: 'humaan-joint-feedback-v1', jointIds: ['joint'], gain: [1.2], damping: [.1], target: [.2] }, warnings: ['Fixed root'], frames: [0,.05,.1].map(time => ({ time, joints: { joint: time }, poses: Object.fromEntries(robot.links.map(l => [l.id,{ position: [0,0,time], quaternion: [0,0,0,1] }])) })) } };
          catalog.runs.push(run); save(catalog); return run;
        }
        throw new Error('Unexpected IPC '+command);
      }
    };
  });
  async function openTraining() {
    await page.getByRole('button', { name: 'Open project', exact: true }).click();
    await page.getByRole('button', { name: 'Experiences', exact: true }).first().click();
    await page.getByRole('button', { name: 'Loops', exact: true }).click();
    await page.getByRole('tab', { name: 'Training experiments' }).click();
  }
  await page.goto('/'); await openTraining();
  await page.locator('summary').filter({ hasText: /^New experience$/ }).click();
  await page.getByRole('textbox', { name: 'New experience name' }).fill('Joint control');
  await page.getByRole('textbox', { name: 'New experience name' }).press('Enter');
  await expect(page.getByRole('combobox', { name: 'Experience', exact: true })).not.toHaveValue('');
  await page.locator('summary').filter({ hasText: /^New loop$/ }).click();
  await page.getByRole('textbox', { name: 'New loop name' }).fill('Baseline loop');
  await page.getByRole('textbox', { name: 'New loop name' }).press('Enter');
  await expect(page.getByRole('combobox', { name: 'Loop', exact: true })).not.toHaveValue('');
  await page.locator('summary').filter({ hasText: /^New environment$/ }).click();
  await page.getByRole('textbox', { name: 'Environment name', exact: true }).fill('Bench');
  await page.getByRole('button', { name: 'Save environment', exact: true }).click();
  await page.locator('summary').filter({ hasText: /^New task$/ }).click();
  await page.getByRole('textbox', { name: 'Task name', exact: true }).fill('Joint reach');
  await page.getByRole('button', { name: 'Save task', exact: true }).click();
  await page.getByRole('textbox', { name: 'Experiment name' }).fill('Arm baseline');
  await page.screenshot({path:'/tmp/humaan-training-setup.png'});
  await page.getByRole('button', { name: 'Save experiment', exact: true }).click();
  await expect(page.getByText('CEM · 3 iterations × 8 candidates · seed 42')).toBeVisible();
  await page.getByRole('button', { name: 'Launch training', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Arm baseline · completed' })).toBeVisible();
  await expect(page.getByText('80.0%', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play evaluation', exact: true }).click();
  await expect(page.getByRole('slider', { name: 'Training evaluation timeline' })).toHaveValue('2');
  await page.screenshot({path:'/tmp/humaan-training-results.png'});
  await page.getByRole('button', { name: 'Manage policy in Deployments', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Add trained policy' })).toBeVisible();
  await expect(page.getByLabel('Policy training run')).not.toHaveValue('');
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.getByRole('button', { name: 'Experiences', exact: true }).first().click();
  await page.getByRole('button', { name: 'Loops', exact: true }).click();
  await page.getByRole('tab', { name: 'Training experiments' }).click();
  await page.getByRole('button', { name: 'Setup', exact: true }).click();
  await page.getByRole('combobox', { name: 'Compute', exact: true }).selectOption('cloud');
  await page.getByRole('textbox', { name: 'SSH host', exact: true }).fill('user@cloud');
  await page.getByRole('button', { name: 'Launch training', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Arm baseline · completed' })).toBeVisible();
  const launch = await page.evaluate(() => JSON.parse(localStorage.getItem('last-training-launch')!));
  expect(launch.execution).toEqual({kind:'cloud',host:'user@cloud',python:'/usr/bin/python3'});
  await page.getByRole('button', { name: 'Compare', exact: true }).click();
  await expect(page.getByRole('row')).toHaveCount(3);
  await page.getByRole('button', { name: 'Setup', exact: true }).click();
  await page.getByRole('textbox', { name: 'SSH host', exact: true }).fill('unreachable');
  await page.getByRole('button', { name: 'Launch training', exact: true }).click();
  await expect(page.getByRole('alert').filter({ hasText: 'SSH connection failed' })).toBeVisible();
  await page.reload(); await openTraining();
  await expect(page.locator('summary').filter({ hasText: 'Saved training runs (3)' })).toBeVisible();
  expect(errors).toEqual([]);
});
