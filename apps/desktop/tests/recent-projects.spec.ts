import { test, expect } from './fixtures';
test('recent projects persist, reopen directly, and can be removed without touching files', async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).__TAURI_INTERNALS__ = { transformCallback: () => 1, unregisterCallback: () => {}, invoke: async (command: string, args: any) => {
      if (command === 'select_directory') { localStorage.setItem('picker-count', String(Number(localStorage.getItem('picker-count') || 0) + 1)); return '/test/project'; }
      if (command === 'experience_project') {
        if (localStorage.getItem('missing-project')) throw new Error('Project folder no longer exists.');
        localStorage.setItem('opened-path', args.path);
        return {root:'/test/project',project:{id:'test',name:'Recent robot'}};
      }
      if (command === 'experience_list') return {records:[],warning:null};
      if (command === 'experience_reviews') return {};
      if (command === 'builder_project') return { activeRobotId: null, robots: [] };
      if (command === 'builder_draft_load') return null;
      if (command === 'project_entries') return {entries:[],truncated:false};
      if (command === 'simulation_tools_status') return {ready:false,installing:false,phase:'Not installed'};
      return [];
    }};
  });
  await page.goto('/');
  await expect(page.getByRole('region', {name:'Recent projects'})).toContainText('Projects you open will appear here.');
  await page.getByRole('button', {name:'Open project',exact:true}).click();
  await expect(page.getByRole('button', {name:'Builder',exact:true})).toBeVisible();
  await page.reload();
  await page.getByRole('button', {name:'Open recent project Recent robot',exact:true}).click();
  await expect(page.getByRole('button', {name:'Builder',exact:true})).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('picker-count'))).toBe('1');
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('humaan-recent-projects')!).length)).toBe(1);
  await page.evaluate(() => localStorage.setItem('missing-project','yes'));
  await page.reload();
  await page.getByRole('button', {name:'Open recent project Recent robot',exact:true}).click();
  await expect(page.getByRole('article', {name:'Opening project failed'})).toBeVisible();
  await expect(page.getByRole('button', {name:'Open recent project Recent robot',exact:true})).toBeEnabled();
  await page.getByRole('button', {name:'Remove Recent robot from recent projects',exact:true}).click();
  await page.reload();
  await expect(page.getByRole('region', {name:'Recent projects'})).toContainText('Projects you open will appear here.');
});
