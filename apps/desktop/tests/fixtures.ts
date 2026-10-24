import { test as base, expect } from '@playwright/test';

// A successful interaction must not hide an uncaught application exception.
export const test = base.extend<{ uncaughtErrors: void }>({
  uncaughtErrors: [async ({ page }, use) => {
    const errors: string[] = [];
    const capture = (error: Error) => errors.push(error.stack || error.message);
    page.on('pageerror', capture);
    await use();
    page.off('pageerror', capture);
    expect(errors, 'Uncaught browser errors').toEqual([]);
  }, { auto: true }],
});

export { expect };
export type { Page } from '@playwright/test';
