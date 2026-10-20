import { test, expect } from '@playwright/test';

test('review cost, launch, reconnect, retrieve results and confirm compute cleanup', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => {
    const read = () => JSON.parse(localStorage.getItem('cloud-jobs-test') || '[]');
    const save = (jobs: unknown) => localStorage.setItem('cloud-jobs-test', JSON.stringify(jobs));
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string, args: any) => {
      if (command.startsWith('plugin:event|')) return 1;
      if (command === 'select_directory') return '/test/project';
      if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Cloud test' } };
      if (command === 'experience_list') return { records: [], warning: null };
      if (command === 'experience_reviews') return {};
      if (['learning_set_list', 'simulation_list', 'builder_list'].includes(command)) return [];
      if (command === 'builder_project') return { activeRobotId: null, robots: [] };
      if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
      if (command === 'training_catalog') return { experiences: [], loops: [], tasks: [], environments: [], experiments: [{ id: '123', name: 'Reach arm', robotName: 'Arm' }], runs: [] };
      if (command === 'cloud_connect') return { accountId: 'runpod-account', gpus: [{ id: 'NVIDIA RTX A6000', displayName: 'RTX A6000', memoryInGb: 48, secureCloud: true }] };
      if (command === 'cloud_jobs') return read();
      if (command === 'cloud_quote') { const quote = { id: '456', accountId: 'runpod-account', experimentName: 'Reach arm', hourlyUsd: 1.2, estimatedComputeUsd: .6, imageName: 'runpod/pytorch:test', config: args.config, expiresAt: Date.now()+900000 }; localStorage.setItem('cloud-quote-test', JSON.stringify(quote)); return quote; }
      if (command === 'cloud_launch') { const q = JSON.parse(localStorage.getItem('cloud-quote-test')!); const job = { id: '456', name: 'Reach arm', experimentId: '123', status: 'provisioning', instanceId: 'i-test', instanceState: 'CREATED', config: q.config, accountId: q.accountId, createdAt: Date.now() }; save([job]); return job; }
      if (command === 'cloud_job_action') { const [job] = read(); if (args.action === 'refresh') { job.status = 'completed'; job.instanceState = job.instanceState === 'TERMINATING' ? 'TERMINATED' : 'RUNNING'; job.logs = 'HUMAAN_PROGRESS: iteration 3'; job.remote = { updatedAt: Date.now(), elapsedSeconds: 15, progress: { iteration: 3, loss: .02 }, evaluation: { successRate: .8, meanError: .02 }, gpu: 'NVIDIA A10G, 0 %, 0 MiB, 23028 MiB' }; job.lastCheckedAt = Date.now(); } if (args.action === 'retrieve') job.retrieved = true; if (args.action === 'release') job.instanceState = 'TERMINATING'; save([job]); return job; }
      throw new Error('Unexpected IPC '+command);
    } };
  });
  async function open() { await page.getByRole('button', { name: 'Open project', exact: true }).click(); await page.getByRole('button', { name: 'Compute', exact: true }).click(); await expect(page.getByRole('heading', { name: 'Choose your provider' })).toBeVisible(); await page.getByRole('button', { name: 'Select NVIDIA Brev' }).click(); await expect(page.getByRole('heading', { name: 'NVIDIA Brev' })).toBeVisible(); await page.getByRole('button', { name: 'Back to providers' }).click(); await expect(page.getByRole('img', { name: 'RunPod', exact: true })).toBeVisible(); await page.screenshot({ path: '/tmp/humaan-compute-providers.png' }); await page.getByRole('button', { name: 'Select RunPod' }).click(); await page.getByRole('textbox', { name: 'RunPod API key' }).fill('rpa_testtesttesttesttest'); await page.getByRole('button', { name: 'Connect account' }).click(); }
  await page.goto('/'); await open();
  const workspace = page.getByRole('region', { name: 'Compute workspace' });
  await page.getByRole('button', { name: 'Review cost and launch' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('$0.60 estimated compute');
  await expect(dialog.getByRole('button', { name: 'Provision and launch' })).toBeDisabled();
  await dialog.getByRole('checkbox').check();
  await page.screenshot({ path: '/tmp/humaan-cloud-cost-review.png' });
  await dialog.getByRole('button', { name: 'Provision and launch' }).click();
  await expect(workspace).toContainText('Job · provisioning');
  await page.reload(); await open();
  await page.getByRole('button', { name: 'Reach arm provisioning' }).click();
  await workspace.getByRole('button', { name: 'Refresh cloud job' }).click();
  await expect(workspace).toContainText('Job · completed');
  await expect(workspace).toContainText('Compute · RUNNING');
  await workspace.getByRole('button', { name: 'Logs', exact: true }).click();
  await expect(workspace).toContainText('HUMAAN_PROGRESS: iteration 3');
  await workspace.getByRole('button', { name: 'Overview', exact: true }).click();
  await workspace.getByRole('button', { name: 'Retrieve results', exact: true }).click();
  await expect(workspace).toContainText('Results saved locally');
  await workspace.getByRole('button', { name: 'Release compute' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Terminate pod' }).click();
  await expect(workspace).toContainText('Compute · TERMINATING');
  await workspace.getByRole('button', { name: 'Refresh cloud job' }).click();
  await expect(workspace).toContainText('Compute · TERMINATED');
  await page.screenshot({ path: '/tmp/humaan-cloud-job.png' });
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('rpa_testtesttesttesttest');
  expect(errors).toEqual([]);
});
