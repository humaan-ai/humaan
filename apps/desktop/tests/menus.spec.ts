import { test, expect } from '@playwright/test';

test('menus open settings, gate workspace navigation, toggle panels, and preserve editor drafts', async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).isTauri = true;
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number>();
    let id = 0;
    (window as any).__menu = (event: string, payload = '') => callbacks.get(listeners.get(event)!)?.({event, payload, id: 1});
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: (callback: (event: unknown) => void) => { callbacks.set(++id, callback); return id; },
      unregisterCallback: (id: number) => callbacks.delete(id),
      invoke: async (command: string, args: any) => {
        if (command === 'plugin:event|listen') { listeners.set(args.event, args.handler); return args.handler; }
        if (command.startsWith('plugin:event|')) return;
        if (command === 'set_project_menu_enabled') { (window as any).__menuEnabled = args.enabled; return; }
        if (command === 'uses_custom_titlebar' || command === 'show_titlebar_icon') return false;
        if (command === 'select_directory') return '/test/project';
        if (command === 'experience_project') return { root: '/test/project', project: {id: 'test', name: 'Menu project'} };
        if (command === 'experience_list') return {records: [], warning: null};
        if (command === 'experience_reviews') return {};
        if (command === 'project_entries') return {truncated: false, entries: [{name: 'robot.txt', path: 'robot.txt', directory: false, size: 8, kind: 'Text'}]};
        if (command === 'project_read_file') return {content: 'original', readOnly: false};
        return [];
      },
    };
  });
  const menu = (event: string, payload = '') => page.evaluate(([event, payload]) => (window as any).__menu(event, payload), [event, payload]);
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => (window as any).__menuEnabled)).toBe(false);
  await menu('studio:menu-action', 'view-builder');
  await expect(page.getByRole('button', {name: 'Open project', exact: true})).toBeVisible();
  await menu('studio:settings');
  await expect(page.getByRole('tab', {name: 'General', exact: true})).toHaveAttribute('aria-selected', 'true');
  await page.getByRole('button', {name: 'Close settings', exact: true}).click();
  await menu('studio:menu-action', 'toggle-chat');
  await expect(page.locator('#chat-toggle')).toHaveAttribute('aria-expanded', 'true');
  await menu('studio:menu-action', 'toggle-chat');
  await menu('studio:menu-action', 'toggle-terminal');
  await expect(page.locator('#terminal-toggle')).toHaveAttribute('aria-expanded', 'true');
  await menu('studio:menu-action', 'toggle-terminal');
  await menu('studio:open-project');
  await expect.poll(() => page.evaluate(() => (window as any).__menuEnabled)).toBe(true);
  await menu('studio:menu-action', 'view-project');
  const workspace = page.getByRole('region', {name: 'Project workspace'});
  await workspace.getByRole('button', {name: 'Files Browse and edit source files'}).click();
  await page.getByRole('complementary', {name: 'Project', exact: true}).getByRole('button', {name: 'robot.txt', exact: true}).click();
  const editor = workspace.getByRole('textbox', {name: 'Editor robot.txt'});
  await editor.fill('unsaved draft');
  await menu('studio:menu-action', 'edit-undo');
  await expect(editor).toHaveText('original');
  await menu('studio:menu-action', 'edit-redo');
  await expect(editor).toHaveText('unsaved draft');
  await menu('studio:open-recent');
  await page.getByRole('dialog').getByRole('button', {name: 'Open recent project Menu project', exact: true}).click();
  await expect(page.getByText('Unsaved drafts', {exact: true}).first()).toBeVisible();
  await expect(page.getByRole('dialog', {name: 'Open recent project', exact: true})).toBeVisible();
  await page.getByRole('dialog').getByRole('button', {name: 'Close', exact: true}).click();
  await expect(editor).toHaveText('unsaved draft');
  await menu('studio:menu-action', 'view-catalog');
  await expect(page.getByRole('button', {name: 'Catalog', exact: true})).toHaveAttribute('aria-current', 'page');
  await menu('studio:menu-action', 'view-project');
  await expect(editor).toHaveText('unsaved draft');
});
