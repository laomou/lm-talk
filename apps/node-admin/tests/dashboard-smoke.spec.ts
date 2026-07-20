import { expect, test, type Page } from '@playwright/test'

const NODE_ORIGIN = 'http://node.test'

async function installMockNode(page: Page) {
  await page.route(`${NODE_ORIGIN}/**`, async (route) => {
    const url = new URL(route.request().url())
    if (url.pathname === '/api/health') {
      return route.fulfill({
        json: {
          status: 'ok',
          peer_id: 'peer-abc',
          node_id: 'node-xyz',
          peers: 2,
          prekeys: 5,
          mailbox_deliveries: 3,
          mailbox_bytes: 1024,
          mailbox_max_bytes: 10485760,
          state_db_encrypted: true,
          state_db_permissions_hardened: true,
        },
      })
    }
    if (url.pathname === '/api/control/stats') {
      return route.fulfill({
        json: {
          requests_total: 42,
          responses_2xx: 40,
          responses_4xx: 2,
          responses_5xx: 0,
          unauthorized: 0,
          rate_limited: 0,
          cors_rejected: 0,
          bad_requests: 0,
          endpoints: { '/api/health': { requests: 30, responses_2xx: 30 } },
        },
      })
    }
    if (url.pathname === '/api/sync/status') {
      return route.fulfill({
        json: {
          peers: {
            'http://peer-ok': { url: 'http://peer-ok', attempts: 5, successes: 5, failures: 0, consecutive_failures: 0 },
            'http://peer-bad': { url: 'http://peer-bad', attempts: 4, successes: 1, failures: 3, consecutive_failures: 2, last_error: 'timeout' },
          },
        },
      })
    }
    if (url.pathname === '/api/dht/maintenance') {
      return route.fulfill({
        json: { peers: 2, records: 4, routing_peers: 3, replication: {}, routing_refresh: {} },
      })
    }
    return route.fulfill({ json: { ok: true } })
  })
}

test('节点管理面板可连接并展示健康与运行 DHT 维护', async ({ page }) => {
  await installMockNode(page)
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'LM Node Admin' })).toBeVisible()

  // Connect to the mock node.
  await page.getByLabel('节点地址').fill(NODE_ORIGIN)
  await page.getByLabel('控制面令牌').fill('secret-token')
  await page.getByRole('button', { name: '连接', exact: true }).click()
  await expect(page.locator('.sync-pill').filter({ hasText: '已连接' })).toBeVisible()

  // Health panel renders the mocked status.
  await expect(page.getByText('ok', { exact: true })).toBeVisible()
  await expect(page.getByText('peers 2', { exact: true })).toBeVisible()

  // Stats and federation peer summary panels load.
  await expect(page.getByText('请求总数 42', { exact: true })).toBeVisible()
  await expect(page.getByText('联邦 peer')).toBeVisible()
  await expect(page.getByText('健康 1', { exact: true })).toBeVisible()
  await expect(page.getByText('异常 1', { exact: true })).toBeVisible()
  await expect(page.getByText('http://peer-bad', { exact: true })).toBeVisible()

  // Trigger DHT maintenance and confirm the request is issued + result rendered.
  const maintenanceRequest = page.waitForRequest(`${NODE_ORIGIN}/api/dht/maintenance*`)
  await page.getByRole('button', { name: '运行 DHT 维护' }).click()
  await maintenanceRequest
  await expect(page.getByLabel('DHT 运维结果')).toContainText('"records": 4')
})
