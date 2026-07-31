import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  timeout: 180_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  // The browser flow suite exercises real two-user mailbox timing. A single
  // retry in CI absorbs transient runner/network stalls without changing the
  // local developer feedback loop.
  retries: process.env.CI ? 1 : 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:4173',
    locale: 'zh-CN',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'node tests/support/web-e2e-server.mjs',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: false,
    timeout: 180_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
})
