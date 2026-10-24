import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  testMatch: '**/*.spec.ts',
  // Software-rendered 3D views compete for CPU on hosted runners.
  workers: process.env.CI ? 1 : undefined,
  use: { baseURL: 'http://127.0.0.1:1420', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure', screenshot: 'only-on-failure', launchOptions: { args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] } },
  webServer: { command: 'bun run dev --host 127.0.0.1', url: 'http://127.0.0.1:1420', reuseExistingServer: !process.env.CI },
});
