import { test, expect } from '@playwright/test';

for (const platform of ['macos', 'windows']) {
  test(`update menu checks from welcome and opens the matching download on ${platform}`, async ({ page }) => {
    await page.addInitScript((platform) => {
      (window as any).isTauri = true;
      const callbacks = new Map<number, (event: unknown) => void>();
      let callbackId = 0;
      (window as any).__TAURI_INTERNALS__ = {
        transformCallback: (callback: (event: unknown) => void) => { callbacks.set(++callbackId, callback); return callbackId; },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (command: string, args: any) => {
          if (command === 'uses_custom_titlebar') return false;
          if (command === 'show_titlebar_icon') return platform !== 'macos';
          if (command === 'studio_update_target') return platform === 'macos' ? 'macos-aarch64' : 'windows-x86_64';
          if (command === 'open_studio_download') { localStorage.setItem('opened-download', args.url); return; }
          if (command === 'select_directory') return '/test/project';
          if (command === 'experience_project') return { root: '/test/project', project: { id: 'test', name: 'Update test' } };
          if (command === 'experience_list') return { records: [], warning: null };
          if (command === 'experience_reviews') return {};
          if (command === 'plugin:event|listen' && args.event === 'studio:check-updates') {
            (window as any).__requestUpdate = () => callbacks.get(args.handler)?.({event: args.event, id: 1, payload: null});
            return 1;
          }
          if (command.startsWith('plugin:event|')) return 1;
          return [];
        },
      };
    }, platform);
    let fail = true;
    const name = `Humaan_0.1.0-alpha.2_${platform === 'macos' ? 'aarch64.dmg' : 'x64-setup.exe'}`;
    const url = `https://github.com/humaan-ai/humaan/releases/download/studio-v0.1.0-alpha.2/${name}`;
    await page.route('https://api.github.com/repos/humaan-ai/humaan/releases?*', route => route.fulfill({
      status: fail ? 503 : 200,
      contentType: 'application/json',
      body: JSON.stringify(fail ? {} : [{ tag_name: 'studio-v0.1.0-alpha.2', draft: false, prerelease: true,
        assets: [{name, state: 'uploaded', size: 100, browser_download_url: url}] }]),
    }));
    await page.goto('/');
    await expect(page.locator('header img[src="/favicon.svg"]')).toHaveCount(platform === 'macos' ? 0 : 1);
    await expect.poll(() => page.evaluate(() => typeof (window as any).__requestUpdate)).toBe('function');
    await page.evaluate(() => (window as any).__requestUpdate());
    await expect(page.getByRole('tab', {name: 'About', exact: true})).toHaveAttribute('aria-selected', 'true');
    await expect(page.locator('#settings-panel-about').getByRole('alert')).toContainText('HTTP 503');
    fail = false;
    await page.getByRole('button', {name: 'Check for updates', exact: true}).click();
    await expect(page.locator('#settings-update-status')).toContainText('Version 0.1.0-alpha.2 is available');
    await page.getByRole('button', {name: 'Download 0.1.0-alpha.2', exact: true}).click();
    await expect.poll(() => page.evaluate(() => localStorage.getItem('opened-download'))).toBe(url);
  });
}
