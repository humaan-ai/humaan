import { test, expect } from './fixtures';

test('policies register controllers, block incompatible assignments, restore history, and replay', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const pose = { xyz: [0,0,0], rpy: [0,0,0] };
    const robot = { schema: 1, name: 'SO-101', assets: {}, links: [{ id: 'base', name: 'base', visuals: [{ kind: 'box', size: [.2,.2,.3], origin: pose, color: '#8ca8c3' }], collisions: [], mass: 1, inertia: [1,0,0,1,0,1], inertial: pose, sensors: [], mounts: [] }], joints: [{ id: 'joint', name: 'Shoulder', type: 'revolute', lower: -1, upper: 1, parent: 'base', child: 'tip', axis: [0,0,1], origin: pose }] };
    const run = (id: string, name: string) => ({ id, name, createdAt: 1780000000000, robotName: 'SO-101', status: 'completed', versionId: '1-2', experiment: { robot, versionId: '1-2', experience: { name: 'Joint reach' }, loop: { name: 'Calibration' }, task: { name: 'Reach target' } }, result: { engine: 'MuJoCo 3.2.7', evaluation: { successRate: .8, meanError: .04 }, policy: { format: 'humaan-joint-feedback-v1', jointIds: ['joint'], gain: [1], damping: [.1], target: [.5] }, frames: [{ time: 0, poses: {} }, { time: .1, poses: {} }] } });
    const runs = [run('run1','Controller v1'), run('run2','Controller v2')]; const policies: any[] = []; const history: any[] = [];
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string, args: any) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/test/project';
      if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Policy workflow' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['builder_list','learning_set_list','simulation_list'].includes(command)) return [];
      if (command === 'builder_project') return { activeRobotId: null, robots: [] };
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      if (command === 'deployment_list') return args.kind === 'robots' ? [{ id: 'robot', name: 'SO-101 bench' },{ id: 'wrong', name: 'Different robot' }] : [];
      if (command === 'policy_list') return policies;
      if (command === 'policy_load') return policies.find(p => p.id === args.id);
      if (command === 'training_catalog') return { runs, experiences: [], loops: [], environments: [], tasks: [], experiments: [] };
      if (command === 'policy_promote') { const r = runs.find(r => r.id === args.runId)!; const p = { id: crypto.randomUUID(), name: args.name, createdAt: Date.now(), versionId: '1-2', runId: r.id, robotName: r.robotName, run: r }; policies.push(p); return p; }
      if (command === 'policy_assignment') return { robotId: args.robotId, history: args.robotId === 'robot' ? history : [] };
      if (command === 'policy_check') return { compatible: args.robotId === 'robot', issues: args.robotId === 'robot' ? [] : ['Robot version differs from the training version.'], runtime: 'unavailable' };
      if (command === 'policy_assign') {
        const policyId = args.restoreId ? history.find(e => e.id === args.restoreId).policyId : args.policyId;
        const item = policies.find(p => p.id === policyId);
        history.push({ id: crypto.randomUUID(), policyId, policyName: item?.name || null, createdAt: Date.now(), action: args.restoreId ? 'rollback' : policyId ? 'assign' : 'unassign', restoredFrom: args.restoreId });
        return { robotId: args.robotId, history };
      }
      throw new Error('Unexpected IPC ' + command);
    } };
  });
  await page.goto('/'); await page.getByRole('button', { name: 'Open project', exact: true }).click();
  await page.getByRole('button', { name: 'Deployments', exact: true }).click();
  await page.getByRole('navigation', { name: 'Deployment sections' }).getByRole('button', { name: 'Policies', exact: true }).click();
  const workspace = page.getByRole('region', { name: 'Policies workspace' });
  await workspace.getByRole('button', { name: 'Add trained policy' }).click();
  await page.getByLabel('Policy training run').selectOption('run1');
  await page.getByRole('button', { name: 'Save policy', exact: true }).click();
  await expect(workspace.getByRole('heading', { name: 'Controller v1', exact: true })).toBeVisible();
  await workspace.getByLabel('Policy target robot').selectOption('wrong');
  await expect(workspace.getByText('Incompatible configuration', { exact: true })).toBeVisible();
  await expect(workspace.getByRole('button', { name: 'Assign policy', exact: true })).toBeDisabled();
  await workspace.getByLabel('Policy target robot').selectOption('robot');
  await expect(workspace.getByText('Configuration matches', { exact: true })).toBeVisible();
  await workspace.getByRole('button', { name: 'Assign policy', exact: true }).click();
  await expect(workspace.getByRole('button', { name: 'Assigned locally', exact: true })).toBeDisabled();
  await page.getByRole('complementary', { name: 'Deployments', exact: true }).getByRole('button', { name: 'Add trained policy' }).click();
  await page.getByLabel('Policy training run').selectOption('run2');
  await page.getByRole('button', { name: 'Save policy', exact: true }).click();
  await workspace.getByRole('button', { name: 'Assign policy', exact: true }).click();
  await workspace.getByRole('button', { name: 'Restore Controller v1 assignment' }).click();
  await expect(workspace.getByText('rollback ·', { exact: false })).toBeVisible();
  await workspace.getByRole('button', { name: 'Controller', exact: true }).click();
  await expect(workspace.getByRole('cell', { name: 'Shoulder', exact: true })).toBeVisible();
  await workspace.getByRole('button', { name: 'Evaluation replay', exact: true }).click();
  await expect(workspace.getByLabel('Policy replay timeline')).toBeVisible();
  await expect(workspace.locator('canvas')).toBeVisible();
  await workspace.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.screenshot({ path: '/tmp/humaan-policies.png' });
  await workspace.getByRole('button', { name: 'Remove assignment', exact: true }).click();
  await expect(workspace.getByText('None', { exact: true })).toBeVisible();
  await expect(workspace.getByText('Unavailable · Not running', { exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});
