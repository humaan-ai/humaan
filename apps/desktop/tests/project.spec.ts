import { test, expect } from './fixtures';

test('project overview, folder browsing, asset search, and terminal layout', async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (command: string, args: any) => {
        if (command === 'select_directory') return '/test/project';
        if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Inspection robot' } };
        if (command === 'experience_list') return { records: [], warning: null };
        if (command === 'experience_reviews') return {};
        if (command === 'builder_project') return { activeRobotId: null, robots: [] };
        if (command === 'builder_draft_load') return null;
        if (['builder_activate_robot', 'builder_draft_save'].includes(command)) return;
        if (['learning_set_list', 'builder_list', 'simulation_list', 'deployment_list'].includes(command)) return [];
        if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
        if (command === 'project_create_entry') { localStorage.setItem('created-entry', JSON.stringify(args)); return; }
        if (command === 'project_read_file') return { content: localStorage.getItem('file-content') || 'solid base\nendsolid base', readOnly: false };
        if (command === 'project_save_file') { localStorage.setItem('file-content', args.content); return; }
        if (command === 'project_entries') return { truncated: false, entries: args.assets || args.directory === 'meshes'
          ? [{ name: 'base.stl', path: 'meshes/base.stl', directory: false, size: 2048, kind: 'Mesh' }]
          : [{ name: 'meshes', path: 'meshes', directory: true, size: 0, kind: 'Folder' }] };
        if (command.startsWith('plugin:event|')) return 1;
        throw new Error('Unexpected IPC: ' + command);
      },
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  const workspace = page.getByRole('region', { name: 'Project workspace' });
  const explorer = page.getByRole('complementary', { name: 'Project', exact: true });
  const menu = page.getByRole('navigation', { name: 'Project sections' });
  await expect(workspace.getByRole('heading', { name: 'Inspection robot' })).toBeVisible();
  await expect(workspace.getByRole('heading', { name: 'Give your robot a starting point' })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-overview-empty.png' });
  await workspace.getByRole('complementary', { name: 'Project resources' }).getByRole('button', { name: 'Files Browse and edit source files' }).click();
  await explorer.getByRole('button', { name: 'meshes', exact: true }).click();
  await explorer.getByRole('button', { name: 'base.stl', exact: true }).click();
  const editor = workspace.getByRole('textbox', { name: 'Editor base.stl' });
  await expect(editor).toHaveText('solid base\nendsolid base', { useInnerText: true });
  await expect(workspace.getByRole('heading', { name: 'Files', exact: true })).toHaveCount(0);
  await editor.fill('solid edited\nendsolid edited');
  await page.getByRole('button', { name: 'Builder', exact: true }).click();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(editor).toHaveText('solid edited\nendsolid edited', { useInnerText: true });
  await expect(workspace.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
  await editor.press('Control+s');
  await expect(workspace.getByRole('button', { name: 'base.stl', exact: true })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('file-content'))).toBe('solid edited\nendsolid edited');
  await editor.fill('unsaved');
  await workspace.getByRole('button', { name: 'Close base.stl' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Keep editing' }).click();
  await expect(editor).toHaveText('unsaved');
  await workspace.getByRole('button', { name: 'Close base.stl' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Discard changes' }).click();
  await expect(editor).toHaveCount(0);
  await explorer.getByRole('button', { name: 'meshes', exact: true }).click();
  await expect(explorer.getByRole('button', { name: 'base.stl', exact: true })).toHaveCount(0);
  await explorer.getByRole('button', { name: 'meshes', exact: true }).click();
  await expect(explorer.getByRole('button', { name: 'base.stl', exact: true })).toBeVisible();
  await expect(explorer.getByRole('button', { name: 'meshes', exact: true })).toBeVisible();
  await menu.getByRole('button', { name: 'Assets', exact: true }).click();
  await expect(explorer.getByRole('button', { name: /base.stl/ })).toBeVisible();
  await explorer.getByRole('textbox', { name: 'Search assets' }).fill('missing');
  await expect(explorer.getByText('No matching files.')).toBeVisible();
  await explorer.getByRole('textbox', { name: 'Search assets' }).fill('');
  await page.getByRole('button', { name: 'Show bottom panel', exact: true }).click();
  const sidebar = await page.getByRole('complementary', { name: 'Project', exact: true }).boundingBox();
  const terminal = await page.getByRole('region', { name: 'Bottom panel', exact: true }).boundingBox();
  expect(terminal!.x).toBeGreaterThanOrEqual(sidebar!.x + sidebar!.width);
  await explorer.getByRole('button', { name: /base.stl/ }).click();
  await expect(editor).toHaveText('solid edited\nendsolid edited', { useInnerText: true });
  await expect(workspace.locator('.cm-editor')).toHaveCSS('background-color', 'rgb(18, 19, 20)');
  await expect(workspace.locator('.cm-lineNumbers')).toContainText('2');
  await menu.getByRole('button', { name: 'Files', exact: true }).click();
  await explorer.getByRole('button', { name: 'meshes', exact: true }).click();
  await explorer.hover();
  await explorer.getByRole('button', { name: 'New file', exact: true }).click();
  await explorer.getByRole('textbox', { name: 'New file name' }).fill('discard.txt');
  await workspace.locator('header').click({ position: { x: 600, y: 20 } });
  await expect(explorer.getByRole('textbox', { name: 'New file name' })).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem('created-entry'))).toBeNull();
  await explorer.hover();
  await explorer.getByRole('button', { name: 'New file', exact: true }).click();
  await explorer.getByRole('textbox', { name: 'New file name' }).fill('robot.urdf');
  await explorer.getByRole('textbox', { name: 'New file name' }).press('Enter');
  const robotEditor = workspace.getByRole('textbox', { name: 'Editor robot.urdf' });
  await expect(robotEditor).toBeVisible();
  await robotEditor.fill('<robot name="test">\n  <link name="base"/>\n</robot>');
  await expect(robotEditor.locator('.cm-line span').first()).toBeVisible();
  await expect(workspace.locator('.cm-lineNumbers')).toContainText('3');
  await robotEditor.press('Control+s');
  const folderIcon = await explorer.getByRole('button', { name: 'New folder', exact: true }).boundingBox();
  const refreshIcon = await explorer.getByRole('button', { name: 'Refresh files', exact: true }).boundingBox();
  expect(refreshIcon!.x).toBeGreaterThan(folderIcon!.x);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('created-entry')!))).toMatchObject({ name: 'robot.urdf', directory: false });
  await explorer.hover();
  await explorer.getByRole('button', { name: 'New folder', exact: true }).click();
  await explorer.getByRole('textbox', { name: 'New folder name' }).fill('models');
  await explorer.getByRole('textbox', { name: 'New folder name' }).press('Enter');
  await expect(explorer.getByRole('textbox', { name: 'New folder name' })).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('created-entry')!))).toMatchObject({ name: 'models', directory: true });
  const minimap = workspace.getByRole('slider', { name: 'Editor minimap' });
  await expect(minimap).toBeVisible();
  await robotEditor.fill(Array.from({ length: 200 }, (_, index) => `<link name="part${index}"/>`).join('\n'));
  await minimap.focus(); await minimap.press('End');
  await expect(minimap).toHaveAttribute('aria-valuenow', '100');
  await workspace.getByRole('button', { name: 'Preview mode' }).click();
  await expect(workspace.getByRole('region', { name: 'Preview robot.urdf' })).toContainText('part199');
  await workspace.getByRole('button', { name: 'Preview mode' }).click();
  await expect(robotEditor).toContainText('part0');
  await explorer.hover();
  await explorer.getByRole('button', { name: 'New file', exact: true }).click();
  await explorer.getByRole('textbox', { name: 'New file name' }).fill('README.md');
  await explorer.getByRole('textbox', { name: 'New file name' }).press('Enter');
  const markdownEditor = workspace.getByRole('textbox', { name: 'Editor README.md' });
  await markdownEditor.fill('# Robot project\n\n**Ready** to build. <script>window.badPreview = true</script>');
  await workspace.getByRole('button', { name: 'Preview mode' }).click();
  const preview = workspace.getByRole('region', { name: 'Preview README.md' });
  await expect(preview.getByRole('heading', { name: 'Robot project' })).toBeVisible();
  await expect(preview.locator('script')).toHaveCount(0);
  await workspace.getByRole('button', { name: 'Preview mode' }).click();
  await expect(markdownEditor).toContainText('# Robot project');
  await page.screenshot({ path: '/tmp/humaan-project.png' });
});

test('file context menu confirms deletion, preserves edits on failure, and refreshes files and assets', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const files: Record<string, string> = {
      'meshes/base.stl': 'solid base\nendsolid base',
      'meshes/keep.stl': 'solid keep\nendsolid keep',
      '.gitignore': 'target/',
      '.humaan/project.json': '{}',
    };
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (command: string, args: any) => {
        if (command === 'select_directory') return '/test/project';
        if (command === 'experience_project') return { root: '/test/project', project: { id: 'delete', name: 'File actions' } };
        if (command === 'experience_list') return { records: [], warning: null };
        if (command === 'experience_reviews') return {};
        if (command === 'builder_project') return { activeRobotId: null, robots: [] };
        if (command === 'builder_draft_load') return null;
        if (['learning_set_list', 'builder_list', 'simulation_list', 'deployment_list'].includes(command)) return [];
        if (command === 'simulation_tools_status') return { ready: false, installing: false, phase: 'Not installed' };
        if (command === 'project_read_file') return { content: files[args.path], readOnly: args.path.startsWith('.humaan/') };
        if (command === 'project_save_file') { localStorage.setItem('unexpected-save', args.path); return; }
        if (command === 'project_delete_file') {
          localStorage.setItem('delete-request', JSON.stringify(args));
          if (localStorage.getItem('fail-delete')) throw new Error('Permission denied');
          if (localStorage.getItem('hold-delete')) await new Promise<void>(resolve => { (window as any).finishDelete = resolve; });
          delete files[args.path];
          localStorage.setItem('remaining-files', JSON.stringify(files));
          return;
        }
        if (command === 'project_entries') {
          const paths = args.assets ? Object.keys(files).filter(path => path.endsWith('.stl'))
            : args.directory ? Object.keys(files).filter(path => path.startsWith(args.directory + '/'))
            : ['.humaan', 'meshes', ...Object.keys(files).filter(path => !path.includes('/'))];
          return { truncated: false, entries: paths.map(path => ({
            name: path.split('/').at(-1), path, directory: !(path in files), size: files[path]?.length ?? 0,
            kind: path.endsWith('.stl') ? 'Mesh' : path in files ? 'File' : 'Folder',
          })) };
        }
        if (command.startsWith('plugin:event|')) return 1;
        throw new Error('Unexpected IPC: ' + command);
      },
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Open project', exact: true }).click();
  const explorer = page.getByRole('complementary', { name: 'Project', exact: true });
  const workspace = page.getByRole('region', { name: 'Project workspace' });
  await page.getByRole('navigation', { name: 'Project sections' }).getByRole('button', { name: 'Files', exact: true }).click();
  await expect(explorer.getByRole('button', { name: '.gitignore', exact: true })).toBeVisible();
  const meshes = explorer.getByRole('button', { name: 'meshes', exact: true });
  await meshes.click();
  await explorer.getByRole('button', { name: 'keep.stl', exact: true }).click();
  const file = explorer.getByRole('button', { name: 'base.stl', exact: true });
  await file.click();
  const editor = workspace.getByRole('textbox', { name: 'Editor base.stl' });
  await editor.fill('unsaved robot mesh');
  await file.click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Delete', exact: true })).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-project-file-menu.png' });
  await page.getByRole('menuitem', { name: 'Delete', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Delete file?' });
  await expect(dialog).toContainText('meshes/base.stl');
  await expect(dialog).toContainText('unsaved changes');
  await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(file).toBeFocused();
  await expect(editor).toHaveText('unsaved robot mesh');
  expect(await page.evaluate(() => localStorage.getItem('delete-request'))).toBeNull();

  await page.evaluate(() => localStorage.setItem('fail-delete', 'yes'));
  await file.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Delete', exact: true }).click();
  await dialog.getByRole('button', { name: 'Delete file', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Permission denied');
  await expect(editor).toHaveText('unsaved robot mesh');
  await expect(file).toBeVisible();
  await page.screenshot({ path: '/tmp/humaan-project-delete-error.png' });
  await page.evaluate(() => { localStorage.removeItem('fail-delete'); localStorage.setItem('hold-delete', 'yes'); });
  await dialog.getByRole('button', { name: 'Delete file', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Deleting…', exact: true })).toBeDisabled();
  await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
  await page.keyboard.press('Escape');
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Control+s');
  expect(await page.evaluate(() => localStorage.getItem('unexpected-save'))).toBeNull();
  await page.evaluate(() => { localStorage.removeItem('hold-delete'); (window as any).finishDelete(); });
  await expect(dialog).not.toBeVisible();
  await expect(file).toHaveCount(0);
  await expect(meshes).toHaveAttribute('aria-expanded', 'true');
  await expect(workspace.getByRole('button', { name: 'base.stl', exact: true })).toHaveCount(0);
  await expect(workspace.getByRole('textbox', { name: 'Editor keep.stl' })).toBeVisible();
  await expect(explorer.getByRole('button', { name: 'Refresh files', exact: true })).toBeFocused();
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('delete-request')!))).toEqual({ root: '/test/project', path: 'meshes/base.stl' });
  const remaining = await page.evaluate(() => JSON.parse(localStorage.getItem('remaining-files')!));
  expect(remaining).not.toHaveProperty('meshes/base.stl');
  expect(remaining['meshes/keep.stl']).toBe('solid keep\nendsolid keep');

  // Protected project metadata stays browsable, with deletion disabled.
  await explorer.getByRole('button', { name: '.humaan', exact: true }).click();
  await explorer.getByRole('button', { name: 'project.json', exact: true }).click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Delete', exact: true })).toHaveAttribute('aria-disabled', 'true');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menuitem', { name: 'Delete', exact: true })).not.toBeVisible();

  // The keyboard context menu also supports deleting a file that has no editor tab.
  const dotfile = explorer.getByRole('button', { name: '.gitignore', exact: true });
  await dotfile.focus();
  await dotfile.press('Shift+F10');
  await page.getByRole('menuitem', { name: 'Delete', exact: true }).click();
  await expect(dialog).not.toContainText('unsaved changes');
  await dialog.getByRole('button', { name: 'Delete file', exact: true }).click();
  await expect(dotfile).toHaveCount(0);
  await expect(workspace.getByRole('textbox', { name: 'Editor keep.stl' })).toBeVisible();

  await page.getByRole('navigation', { name: 'Project sections' }).getByRole('button', { name: 'Assets', exact: true }).click();
  await explorer.getByRole('button', { name: /keep.stl/ }).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Delete', exact: true }).click();
  await dialog.getByRole('button', { name: 'Delete file', exact: true }).click();
  await expect(explorer.getByText(/No asset files found/)).toBeVisible();
  await expect(workspace.getByRole('textbox')).toHaveCount(0);
  expect(errors).toEqual([]);
});
