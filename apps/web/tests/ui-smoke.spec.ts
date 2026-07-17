import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

async function clearBrowserState(page: Page) {
  await page.goto('/')
  await page.evaluate(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0))
    localStorage.clear()
    sessionStorage.clear()
    const dbs = await indexedDB.databases?.()
    if (dbs) {
      await Promise.all(dbs.map((db) => db.name ? new Promise<void>((resolve, reject) => {
        const req = indexedDB.deleteDatabase(db.name!)
        req.onsuccess = () => resolve()
        req.onerror = () => reject(req.error)
        req.onblocked = () => resolve()
      }) : Promise.resolve()))
    }
  })
}

function fieldAfterLabel(page: Page, labelText: string, tag = 'textarea'): Locator {
  return page
    .locator(`label:has-text("${labelText}") + ${tag}, label:has-text("${labelText}") + .inline-field ${tag}`)
    .first()
}

type MockMailboxMessage = { delivery_id: string; message: any }

function decodePrefixedJson(text: string): any {
  const payload = text.includes(':') ? text.slice(text.indexOf(':') + 1) : text
  const normalized = payload.replace(/-/g, '+').replace(/_/g, '/')
  const padded = normalized + '='.repeat((4 - normalized.length % 4) % 4)
  return JSON.parse(Buffer.from(padded, 'base64').toString('utf8'))
}





test('self-sync v1 test vector keeps lightweight sync wire shape', async () => {
  const vectorPath = resolve(process.cwd(), '../../test-vectors/self_sync_v1.json')
  const vector = JSON.parse(readFileSync(vectorPath, 'utf8'))
  const pkg = vector.self_sync_package
  expect(pkg.type).toBe('lm-self-sync-v1')
  expect(pkg.version).toBe(1)
  expect(pkg.sync_id).toBeTruthy()
  expect(pkg.sequence).toBeGreaterThan(0)
  expect(pkg.previous_sync_id).toBeTruthy()
  expect(pkg.signature).toBeTruthy()
  expect(Array.isArray(pkg.contacts)).toBeTruthy()
  expect(pkg.myDeviceId).toBe(pkg.from_device_id)
  expect(pkg.messageReceiptStates).toHaveLength(1)
  expect(pkg.messageReceiptStates[0].status).toBe('read')
  expect(pkg.messageReceiptStates[0].mailbox_delivery_id).toBeTruthy()
  expect(pkg.outboxSummary.queued).toBeGreaterThanOrEqual(0)
  expect(pkg.outboxSummary.failed_kinds['direct-envelope']).toBe(1)

  const req = vector.self_sync_request
  expect(req.type).toBe('lm-self-sync-request-v1')
  expect(req.version).toBe(1)
  expect(req.missing_sync_id).toBe(pkg.previous_sync_id)
  expect(req.signature).toBeTruthy()
})

test('per-device envelope v1 test vector keeps sealed slot wire shape', async () => {
  const vectorPath = resolve(process.cwd(), '../../test-vectors/per_device_envelope_v1.json')
  const vector = JSON.parse(readFileSync(vectorPath, 'utf8'))
  expect(vector.type).toBe('lm-per-device-envelope-v1')
  expect(vector.version).toBe(1)
  expect(vector.signature).toBeTruthy()
  expect(vector.target_devices).toHaveLength(2)
  const sealed = vector.target_devices.find((item: any) => item.crypto === 'x25519-ephemeral-hkdf-xchacha20poly1305-device-slot-v1')
  expect(sealed).toBeTruthy()
  expect(sealed.x25519_ephemeral_public_key).toBeTruthy()
  expect(sealed.slot_id).toBeTruthy()
  expect(sealed.nonce).toBeTruthy()
  expect(JSON.parse(sealed.aad).target_device_id).toBe(sealed.device_id)
  const legacy = vector.target_devices.find((item: any) => item.crypto === 'placeholder-shared-envelope-v1')
  expect(legacy).toBeTruthy()
  expect(legacy.x25519_ephemeral_public_key).toBeUndefined()
})

async function installMockSyncNode(context: BrowserContext, mailboxes: Map<string, MockMailboxMessage[]>) {
  const deliveryStatus = (globalThis as any).__lmMockDeliveryStatus ?? new Map<string, string>()
  ;(globalThis as any).__lmMockDeliveryStatus = deliveryStatus
  const syncPeerHealth = (globalThis as any).__lmMockSyncPeerHealth ?? {
    peers: {
      'http://peer-ok': { url: 'http://peer-ok', consecutive_failures: 0, failures: 0 },
      'http://peer-bad': { url: 'http://peer-bad', consecutive_failures: 5, failures: 5, next_attempt_at: Math.floor(Date.now() / 1000) + 600, last_error: 'synthetic DHT failure' },
    },
  }
  ;(globalThis as any).__lmMockSyncPeerHealth = syncPeerHealth
  await context.route('**/*', async (route) => {
    const req = route.request()
    const url = new URL(req.url())
    if (url.hostname !== 'sync.test' && url.hostname !== 'mailbox.test') return route.continue()
    if (url.pathname === '/health') return route.fulfill({ json: { ok: true, peers: 2, prekeys: 1, mailbox_deliveries: 0, mailbox_bytes: 0, mailbox_max_bytes: 10485760, mailbox_max_bytes_per_user: 2097152, mailbox_max_messages_per_user: 1000 } })
    if (url.pathname === '/prekey/publish') return route.fulfill({ json: { ok: true } })
    if (url.pathname === '/sync/status') return route.fulfill({ json: syncPeerHealth })
    if (url.pathname === '/dht/key') {
      const requested = url.searchParams.get('kind')
      const kind = requested === 'mailbox-hint' ? 'MailboxHint' : requested === 'public-peer' ? 'PublicPeer' : 'PreKey'
      return route.fulfill({ json: { kind, value: url.searchParams.get('value'), key: 'b'.repeat(64) } })
    }
    if (url.pathname === '/dht/record') return route.fulfill({ json: { stored: true, inserted: true, key: 'b'.repeat(64), records: 1 } })
    if (url.pathname === '/dht/find-value') {
      const key = url.searchParams.get('key') || 'b'.repeat(64)
      const kindParam = url.searchParams.get('kind')
      const expired = key === 'c'.repeat(64)
      const record = kindParam === 'mailbox-hint'
        ? { key, kind: 'MailboxHint', value: 'http://mailbox.test', expires_at: Math.floor(Date.now() / 1000) + 3600 }
        : { key, kind: 'PreKey', value: 'lm-prekey-bundle-v1:mock-dht-prekey', expires_at: expired ? 1 : Math.floor(Date.now() / 1000) + 3600 }
      return route.fulfill({ json: { key, found: true, record, records: 1, stats: { attempts: 2, successes: 2, failures: 0, found_records: 1, closer_records: 0, peers_quarantined: 0 } } })
    }
    if (url.pathname === '/dht/maintenance') return route.fulfill({ json: { peers: 2, records: 4, routing_peers: 3, replication: { records: 4, attempts: 2, successes: 2, failures: 0, peers_quarantined: 0 }, routing_refresh: { targets: 8, attempts: 2, successes: 2, failures: 0, nodes_returned: 3, nodes_merged: 1, peers_quarantined: 0 } } })
    if (url.pathname === '/dht/replicate') return route.fulfill({ json: { peers: 2, records: 4, stats: { records: 4, attempts: 2, successes: 2, failures: 0, peers_quarantined: 0 } } })
    if (url.pathname === '/dht/routing-refresh') return route.fulfill({ json: { peers: 2, routing_peers: 3, stats: { targets: 8, attempts: 2, successes: 2, failures: 0, nodes_returned: 3, nodes_merged: 1, peers_quarantined: 0 } } })
    if (url.pathname === '/sync/peer/reset') {
      const body = req.postDataJSON() as { url?: string }
      const peer = body.url ? syncPeerHealth.peers[body.url] : undefined
      if (peer) {
        peer.consecutive_failures = 0
        peer.last_error = null
        peer.last_error_at = null
        peer.next_attempt_at = null
      }
      return route.fulfill({ json: { url: body.url, reset: Boolean(peer), status: peer ?? null } })
    }
    if (url.pathname === '/mailbox/ack') {
      const body = req.postDataJSON() as { user_id?: string; delivery_ids?: string[] }
      for (const id of body.delivery_ids ?? []) deliveryStatus.set(`${body.user_id || ''}:${id}`, 'acked')
      return route.fulfill({ json: { ok: true, pending_bytes: 0, max_bytes_per_user: 2097152 } })
    }
    if (url.pathname === '/mailbox/status') {
      const userId = url.searchParams.get('user_id') || ''
      const deliveryId = url.searchParams.get('delivery_id') || ''
      const status = deliveryStatus.get(`${userId}:${deliveryId}`) || 'absent_or_expired'
      return route.fulfill({ json: { user_id: userId, summary: { bytes: 1024 }, max_bytes_per_user: 2097152, delivery: { delivery_id: deliveryId, status } } })
    }
    if (url.pathname === '/mailbox/push') {
      const body = req.postDataJSON() as { message_text: string }
      const message = decodePrefixedJson(body.message_text)
      const deliveryId = `mock-${Date.now()}-${Math.random().toString(36).slice(2)}`
      const key = String(message.to_user_id)
      const queue = mailboxes.get(key) ?? []
      queue.push({ delivery_id: deliveryId, message })
      deliveryStatus.set(`${key}:${deliveryId}`, 'pending')
      if (message.kind === 'direct-envelope') {
        // Simulate ack loss/redelivery from a production mailbox: same protocol
        // message_id, new delivery_id. The client must dedupe by message_id too.
        const redeliveryId = `${deliveryId}-redelivery`
        queue.push({ delivery_id: redeliveryId, message })
        deliveryStatus.set(`${key}:${redeliveryId}`, 'pending')
      }
      mailboxes.set(key, queue)
      return route.fulfill({ json: { delivery_id: deliveryId, pending_bytes: queue.length * 1024, max_bytes_per_user: 2097152 } })
    }
    if (url.pathname === '/mailbox/take') {
      const userId = url.searchParams.get('user_id') || ''
      const limit = Math.max(1, Math.min(Number(url.searchParams.get('limit') || '1') || 1, 1))
      const queued = mailboxes.get(userId) ?? []
      const messages = queued.slice(0, limit)
      for (const item of messages) deliveryStatus.set(`${userId}:${item.delivery_id}`, 'delivered_unacked')
      mailboxes.set(userId, queued.slice(limit))
      return route.fulfill({ json: { messages, returned: messages.length, pending: queued.length, pending_bytes: queued.slice(limit).length * 1024, max_bytes_per_user: 2097152, more: queued.length > messages.length } })
    }
    return route.fulfill({ json: { ok: true } })
  })
}

async function createIdentity(page: Page, name: string, passphrase: string) {
  await page.goto('/#/register')
  await expect(page.getByRole('heading', { name: '注册 LM Talk' })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册', exact: true }).last().click({ force: true })
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()
  await expect(page.getByRole('button', { name: '下载身份' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click({ force: true })
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录', exact: true }).last().click({ force: true })
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.goto('/#/me')
  await fieldAfterLabel(page, '显示名', 'input').fill(name)
  await page.locator('.home-card').filter({ hasText: '显示名' }).getByRole('button', { name: '保存' }).click({ force: true })
  await expect(page.locator('.rail-avatar')).toHaveText(name.slice(0, 1).toUpperCase())
  await page.goto('/#/chat')
  await expect(page.locator('.chat-shell')).toBeVisible()
}

async function localIdentityRecord(page: Page): Promise<{ user_id: string; backup_text: string }> {
  return page.evaluate(() => {
    const records = JSON.parse(localStorage.getItem('lm-talk-local-identities-v1') || '[]') as Array<{ user_id: string; backup_text: string }>
    return records[0] ?? { user_id: '', backup_text: '' }
  })
}

async function copyMyContactCard(page: Page): Promise<string> {
  await page.goto('/#/me')
  await page.getByRole('button', { name: '我的名片' }).click()
  await expect(page.locator('.qr-modal')).toBeVisible()
  await page.locator('.qr-modal').getByRole('button', { name: '复制原文' }).click()
  const value = await page.evaluate(() => navigator.clipboard.readText())
  expect(value).toContain('lm-contact-card-v1:')
  await page.locator('.qr-modal').getByRole('button', { name: '关闭' }).click()
  return value
}

async function enableSync(page: Page) {
  await page.goto('/#/me')
  const syncInput = page.locator('#sync-service-input')
  if (!(await syncInput.isVisible())) await page.getByRole('button', { name: /编辑地址\/?令牌/ }).click()
  await syncInput.fill('http://sync.test')
  const button = page.getByRole('button', { name: '开启同步' })
  if (await button.isVisible()) await button.click()
  await expect(page.getByRole('button', { name: '关闭同步' })).toBeVisible()
}



async function clearIdbStores(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await (window as any).flushPersistForTests?.()
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('lm-talk-web-db')
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
    await new Promise<void>((resolve, reject) => {
      const stores = Array.from(db.objectStoreNames)
      const tx = db.transaction(stores, 'readwrite')
      for (const store of stores) tx.objectStore(store).clear()
      tx.oncomplete = () => { db.close(); resolve() }
      tx.onerror = () => reject(tx.error)
    })
  })
}

async function idbStoreEntry(page: Page, storeName: string, key: string): Promise<any> {
  return page.evaluate(async ({ store, id }) => {
    await (window as any).flushPersistForTests?.()
    const records = JSON.parse(localStorage.getItem('lm-talk-local-identities-v1') || '[]') as Array<{ user_id: string }>
    const userId = records[0]?.user_id || ''
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('lm-talk-web-db')
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
    return await new Promise<any>((resolve, reject) => {
      const tx = db.transaction(store, 'readonly')
      const req = tx.objectStore(store).get(`${userId}::${id}`)
      req.onsuccess = () => resolve(req.result ?? null)
      req.onerror = () => reject(req.error)
      tx.oncomplete = () => db.close()
    })
  }, { store: storeName, id: key })
}




async function idbStoreEntryForUser(page: Page, userId: string, storeName: string, key: string): Promise<any> {
  return page.evaluate(async ({ user, store, id }) => {
    await (window as any).flushPersistForTests?.()
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('lm-talk-web-db')
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
    return await new Promise<any>((resolve, reject) => {
      const tx = db.transaction(store, 'readonly')
      const req = tx.objectStore(store).get(`${user}::${id}`)
      req.onsuccess = () => resolve(req.result ?? null)
      req.onerror = () => reject(req.error)
      tx.oncomplete = () => db.close()
    })
  }, { user: userId, store: storeName, id: key })
}

async function idbPutRaw(page: Page, storeName: string, key: string, value: any): Promise<void> {
  await page.evaluate(async ({ store, id, raw }) => {
    await (window as any).flushPersistForTests?.()
    const records = JSON.parse(localStorage.getItem('lm-talk-local-identities-v1') || '[]') as Array<{ user_id: string }>
    const userId = records[0]?.user_id || ''
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('lm-talk-web-db')
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(store, 'readwrite')
      tx.objectStore(store).put(raw, `${userId}::${id}`)
      tx.oncomplete = () => { db.close(); resolve() }
      tx.onerror = () => reject(tx.error)
    })
  }, { store: storeName, id: key, raw: value })
}

async function idbStoreAll(page: Page, storeName: string): Promise<any[]> {
  return page.evaluate(async (store) => {
    await (window as any).flushPersistForTests?.()
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('lm-talk-web-db')
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
    return await new Promise<any[]>((resolve, reject) => {
      const tx = db.transaction(store, 'readonly')
      const req = tx.objectStore(store).getAll()
      req.onsuccess = () => resolve(req.result ?? [])
      req.onerror = () => reject(req.error)
      tx.oncomplete = () => db.close()
    })
  }, storeName)
}

test('登录注册、主界面和诊断页是产品化 UI', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'Alice', '我爱吃菠萝2026')

  await expect(page.locator('.chat-shell')).toBeVisible()
  await expect(page.locator('.sidebar').getByRole('heading', { name: '聊天', exact: true })).toBeVisible()
  await expect(page.locator('.chat-empty-state').getByRole('heading', { name: '选择一个聊天' })).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('复制密文')
  await expect(page.locator('.app-shell')).not.toContainText('手动接收密文')
  await expect(page.locator('.app-shell')).not.toContainText('离线添加')

  await page.goto('/#/contacts')
  await page.getByRole('button', { name: '收件箱' }).click()
  await expect(page.getByRole('button', { name: '同步' })).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('粘贴好友请求')
  await expect(page.locator('.app-shell')).not.toContainText('粘贴群邀请')
  await expect(page.locator('.app-shell')).not.toContainText('生成邀请')

  await page.goto('/#/me')
  await expect(page.getByRole('button', { name: '立即同步' })).toBeVisible()
  await expect(page.getByText('同步状态')).toBeVisible()
  await expect(page.getByRole('button', { name: '诊断工具' })).toBeVisible()
  await expect(page.getByRole('button', { name: '注册后台同步' })).toBeVisible()
  await expect(page.getByText('后台事件只提示打开应用')).toBeVisible()
  await expect(page.getByText('最近后台事件：尚未收到后台事件')).toBeVisible()
  await page.evaluate(() => (window as any).handlePwaBackgroundSyncForTests?.({ type: 'lm-talk-background-sync', tag: 'lm-talk-mailbox-sync' }))
  await expect(page.getByText(/最近后台事件：.*lm-talk-mailbox-sync/)).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('调试页面')
  await expect(page.locator('.app-shell')).not.toContainText('开发协议工具')

  await page.evaluate(() => (window as any).handlePwaBackgroundSyncForTests?.({ type: 'lm-talk-background-sync', tag: 'lm-talk-mailbox-sync' }))
  await page.getByRole('button', { name: '诊断工具' }).click()
  await expect(page.getByRole('heading', { name: '诊断工具' })).toBeVisible()
  await expect(page.getByText('一键诊断')).toBeVisible()
  await page.evaluate(() => {
    ;(window as any).appendLogForTests?.('secret Bearer super-secret-token lm-identity-backup-v1:abcdef lm-message-receipt-v1:receipt-secret lm-prekey-bundle-v1:prekey-secret http://sync.test|node-token')
    ;(window as any).setDhtDiagnosticsForTests?.('DHT 查找：找到', ['2026/07/16 10:00 · DHT 查找：找到'])
  })
  await page.getByRole('button', { name: '生成诊断报告' }).click()
  await page.getByRole('button', { name: '显示预览' }).click()
  await expect.poll(async () => page.locator('textarea.mono').inputValue()).toContain('service_worker')
  const diagnosticJson = await page.locator('textarea.mono').inputValue()
  expect(diagnosticJson).not.toContain('super-secret-token')
  expect(diagnosticJson).not.toContain('node-token')
  expect(diagnosticJson).not.toContain('lm-identity-backup-v1:abcdef')
  expect(diagnosticJson).not.toContain('lm-message-receipt-v1:receipt-secret')
  expect(diagnosticJson).not.toContain('lm-prekey-bundle-v1:prekey-secret')
  expect(diagnosticJson).toContain('lm-message-receipt-v1:[已脱敏]')
  expect(diagnosticJson).toContain('Bearer [已脱敏]')
  expect(diagnosticJson).toContain('background_event_count')
  expect(diagnosticJson).toContain('lm-talk-mailbox-sync')
  expect(diagnosticJson).toContain('dht_peer_health_summary')
  expect(diagnosticJson).toContain('find_value_status')
  expect(diagnosticJson).toContain('operation_history')
  expect(diagnosticJson).toContain('DHT 查找：找到')

  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute('href', '/manifest.webmanifest')
  await expect(page.locator('meta[http-equiv="Content-Security-Policy"]')).toHaveAttribute('content', /default-src 'self'/)
  await expect(page.locator('meta[name="referrer"]')).toHaveAttribute('content', 'no-referrer')
  const swAvailable = await page.evaluate(() => 'serviceWorker' in navigator)
  expect(swAvailable).toBe(true)
})

test('消息同步可完成好友请求和消息收发', async ({ browser }) => {
  const mailboxes = new Map<string, MockMailboxMessage[]>()
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  await installMockSyncNode(aliceContext, mailboxes)
  await installMockSyncNode(bobContext, mailboxes)
  const alice = await aliceContext.newPage()
  const bob = await bobContext.newPage()
  await clearBrowserState(alice)
  await clearBrowserState(bob)

  await createIdentity(alice, 'Alice', 'alice real sync 2026')
  await createIdentity(bob, 'Bob', 'bob real sync 2026')
  const bobCard = await copyMyContactCard(bob)

  await enableSync(alice)
  await enableSync(bob)
  await expect(bob.getByText(/节点健康：.*Mailbox/)).toBeVisible()
  await expect(bob.getByText(/DHT peer：2 个，失败 1，隔离 1/)).toBeVisible()
  await bob.getByRole('button', { name: '重置 http://peer-bad' }).click()
  await expect(bob.getByText(/DHT peer：2 个，失败 0，隔离 0/)).toBeVisible()
  await bob.getByRole('button', { name: '刷新节点健康' }).click()
  await expect(bob.getByText(/节点健康：.*Mailbox/)).toBeVisible()
  await bob.getByRole('button', { name: '我的 PreKey' }).click()
  await expect(bob.getByText('DHT key：已填入 PreKey，点击“派生 key”生成查询 key', { exact: true })).toBeVisible()
  await bob.getByRole('button', { name: '派生并查找' }).click()
  await expect(bob.getByLabel('DHT record key')).toHaveValue('b'.repeat(64))
  await expect(bob.getByText('DHT 查找：找到，kind PreKey，key bbbbbbbbbbbb…，peer 尝试 2，成功 2，失败 0，found 1，closer 0，隔离 0', { exact: true })).toBeVisible()
  await expect(bob.getByText(/DHT 查到 PreKey record，但验签失败/)).toBeVisible()
  await bob.getByRole('button', { name: '我的 MailboxHint' }).click()
  await bob.getByRole('button', { name: '派生并查找' }).click()
  await expect(bob.getByText(/发现 MailboxHint：http:\/\/mailbox.test/)).toBeVisible()
  await bob.getByRole('button', { name: '加入同步服务' }).click()
  await expect(bob.locator('#sync-service-input')).toHaveValue(/http:\/\/mailbox.test/)
  await bob.getByLabel('DHT record key').fill('c'.repeat(64))
  await bob.getByRole('button', { name: '查找 DHT 记录' }).click()
  await expect(bob.locator('small').filter({ hasText: /^DHT 查到 PreKey record，但record 已过期/ })).toBeVisible()
  await bob.getByRole('button', { name: '运行 DHT 维护' }).click()
  await expect(bob.getByText('DHT 维护：peer 2，records 4，复制成功 2/2，刷新成功 2/2，合并 1，隔离 0', { exact: true })).toBeVisible()
  await bob.getByRole('button', { name: '复制 DHT 记录' }).click()
  await expect(bob.getByText('DHT 复制：peer 2，records 4，尝试 2，成功 2，失败 0，隔离 0', { exact: true })).toBeVisible()
  await bob.getByRole('button', { name: '刷新 DHT 路由' }).click()
  await expect(bob.getByText('DHT 路由刷新：peer 2，尝试 2，成功 2，失败 0，返回 3，合并 1，隔离 0', { exact: true })).toBeVisible()
  await bob.getByRole('button', { name: '发布并查 DHT' }).click()
  await expect(bob.getByText(/已发布并完成 DHT 查找/)).toBeVisible()
  await bob.getByRole('button', { name: '发布并查 MailboxHint' }).click()
  await expect(bob.locator('small').filter({ hasText: /^MailboxHint 已发布并完成 DHT 查找/ })).toBeVisible()
  await bob.getByRole('button', { name: '发布并查 PublicPeer' }).click()
  await expect(bob.locator('small').filter({ hasText: /^PublicPeer 已发布并完成 DHT 查找/ })).toBeVisible()
  await bob.getByRole('button', { name: '发布并查全部 DHT' }).click()
  await expect(bob.locator('small').filter({ hasText: /已发布并完成全部 DHT 查找/ })).toBeVisible()
  await bob.getByLabel('当前会话自动发送已读回执').check()

  await alice.goto('/#/contacts')
  await alice.getByRole('button', { name: '添加' }).click()
  await fieldAfterLabel(alice, '对方名片').fill(bobCard)
  await alice.getByRole('button', { name: '添加好友' }).click()
  await alice.goto('/#/chat')
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.contact').filter({ hasText: '等待通过' })).toBeVisible()

  await bob.goto('/#/contacts')
  await bob.getByRole('button', { name: '收件箱' }).click()
  await bob.getByRole('button', { name: '同步' }).click()
  await expect(bob.getByRole('button', { name: '接受' })).toBeVisible()
  await bob.getByRole('button', { name: '接受' }).click()
  await expect(bob.locator('.contact').filter({ hasText: 'Alice' })).toBeVisible()

  await alice.goto('/#/contacts')
  await alice.getByRole('button', { name: '收件箱' }).click()
  await alice.getByRole('button', { name: '同步' }).click()
  await alice.goto('/#/chat')
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.clean-chat-header')).toContainText('好友')
  await alice.getByPlaceholder('输入消息').fill('你好 Bob')
  await alice.getByRole('button', { name: '发送', exact: true }).click()
  await expect(alice.locator('.bubble.out')).toContainText('你好 Bob')

  await bob.goto('/#/contacts')
  await bob.getByRole('button', { name: '同步' }).click()
  await expect(bob.getByText(/Mailbox 容量：/)).toBeVisible()
  await expect.poll(async () => bob.locator('.contact').filter({ hasText: 'Alice' }).count(), { timeout: 15_000 }).toBeGreaterThan(0)
  await bob.goto('/#/contacts')
  await bob.getByRole('button', { name: '同步' }).click()
  await bob.goto('/#/chat')
  await bob.locator('.contact').filter({ hasText: 'Alice' }).click()
  await expect(bob.locator('.bubble.in').filter({ hasText: '你好 Bob' })).toHaveCount(1)
  await alice.goto('/#/contacts')
  await alice.getByRole('button', { name: '收件箱' }).click()
  await alice.getByRole('button', { name: '同步' }).click()
  await alice.goto('/#/chat')
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.bubble.out').filter({ hasText: '你好 Bob' })).toContainText('已读')

  await bob.goto('/#/chat')
  await bob.locator('.contact').filter({ hasText: 'Alice' }).click()
  await bob.getByLabel('当前联系人已读回执策略').selectOption('disabled')
  const aliceIdentity = await localIdentityRecord(alice)
  await expect.poll(async () => (await idbStoreEntry(bob, 'contacts', aliceIdentity.user_id))?.read_receipts).toBe('disabled')

  await alice.getByPlaceholder('输入消息').fill('https://example.test')
  await alice.getByRole('button', { name: '发送', exact: true }).click({ force: true })
  await expect(alice.getByRole('heading', { name: '发送风险内容' })).toBeVisible()
  await alice.getByRole('button', { name: '取消', exact: true }).click({ force: true })
  await expect(alice.getByRole('heading', { name: '发送风险内容' })).not.toBeVisible()
  await expect(alice.getByPlaceholder('输入消息')).toHaveValue('https://example.test')
  await alice.getByPlaceholder('输入消息').fill('')
  await alice.setInputFiles('input[type="file"]', {
    name: 'danger.exe',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('not really an executable'),
  })
  await expect(alice.getByText('危险类型')).toBeVisible()
  await alice.getByRole('button', { name: '发送文件' }).click({ force: true })
  await expect(alice.getByRole('heading', { name: '发送危险类型文件' })).toBeVisible()
  await alice.getByRole('button', { name: '取消', exact: true }).click({ force: true })
  await expect(alice.getByRole('heading', { name: '发送危险类型文件' })).not.toBeVisible()


  await expect.poll(async () => {
    const messages = await idbStoreAll(alice, 'messages')
    return messages.some((m) => m.text?.__lm_enc_v1 === true && JSON.stringify(m).includes('__lm_enc_v1') && !JSON.stringify(m).includes('你好 Bob'))
  }, { timeout: 10_000 }).toBe(true)
  const storedBobContact = await idbStoreEntry(alice, 'contacts', (await localIdentityRecord(bob)).user_id)
  expect(storedBobContact.contact_card_text.__lm_enc_v1).toBe(true)
  expect(JSON.stringify(storedBobContact)).not.toContain('Bob')
  const aliceMeta = await idbStoreEntry(alice, 'meta', 'main')
  expect(aliceMeta.nodeControlUrl.__lm_enc_v1).toBe(true)
  expect(JSON.stringify(aliceMeta)).not.toContain('sync.test')

  await aliceContext.close()
  await bobContext.close()
})


test('同步服务请求超时会失败并提示切换服务', async ({ page }) => {
  await clearBrowserState(page)
  await page.route('http://sync.test/**', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 5_000))
    await route.fulfill({ json: { ok: true } }).catch(() => {})
  })
  await createIdentity(page, 'Timeout', 'timeout sync 2026')
  await page.evaluate(() => { (window as any).nodeFetchTimeoutMsForTests = 100 })
  await enableSync(page)
  await page.getByRole('button', { name: '立即同步' }).click({ force: true })
  await expect(page.getByText('同步服务请求超时，请稍后重试或切换同步服务。', { exact: true })).toBeVisible()
})

test('浏览器注册使用 Web RNG 生成不同身份', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'RngA', 'same rng passphrase 2026')
  const first = await localIdentityRecord(page)

  await clearBrowserState(page)
  await createIdentity(page, 'RngB', 'same rng passphrase 2026')
  const second = await localIdentityRecord(page)

  expect(first.backup_text).toContain('lm-identity-backup-v1:')
  expect(second.backup_text).toContain('lm-identity-backup-v1:')
  expect(first.user_id).toBeTruthy()
  expect(second.user_id).toBeTruthy()
  expect(second.user_id).not.toBe(first.user_id)
})






test('旧 localStorage 状态迁移成功后才删除原始数据', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'Legacy', 'legacy migration 2026')
  const identity = await localIdentityRecord(page)
  await clearIdbStores(page)
  await page.evaluate((backup) => {
    localStorage.setItem('lm-talk-chat-state-v1', JSON.stringify({
      backupText: backup,
      contacts: [],
      friendRequests: [],
      groups: [],
      groupInvites: [],
      groupSenderKeys: [],
      messages: [],
      outbox: [],
      ratchetSessions: [],
      myContactCardText: '',
      nodeControlUrl: 'http://legacy.test',
      nodeEnabled: true,
      autoMailboxTake: true,
      autoPublishPreKey: true,
      autoNodeSync: false,
      processedMailboxIds: [],
      mailboxFailedItems: [],
      syncRecoveryHistory: [],
      friendRequestRateRecords: [],
    }))
  }, identity.backup_text)
  await page.reload()
  await fieldAfterLabel(page, '提示词').fill('legacy migration 2026')
  await page.getByRole('button', { name: '登录', exact: true }).click({ force: true })
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.goto('/#/me')
  await expect(page.getByText('http://legacy.test', { exact: true })).toBeVisible()
  const legacyState = await page.evaluate(() => localStorage.getItem('lm-talk-chat-state-v1'))
  expect(legacyState).toBeNull()
  expect(await idbStoreEntry(page, 'meta', 'main')).not.toBeNull()
})

test('设置页可导出并导入完整数据备份恢复本地同步设置', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'Backup', 'backup roundtrip 2026')
  await enableSync(page)
  await page.goto('/#/me')
  await page.getByLabel('当前会话自动发送已读回执').uncheck()
  await page.evaluate(() => {
    ;(window as any).setDhtDiagnosticsForTests?.('DHT 查找：备份测试', ['2026/07/16 10:10 · DHT 查找：备份测试'])
  })
  await page.getByRole('button', { name: '显示备份文本' }).click()
  await page.getByRole('button', { name: '生成备份' }).click()
  const backupTextArea = page.getByLabel('完整数据备份文本')
  await expect.poll(async () => backupTextArea.inputValue()).toContain('lm-data-backup-v1:')
  const dataBackup = await backupTextArea.inputValue()
  const syncInput = page.locator('#sync-service-input')
  if (!(await syncInput.isVisible())) await page.getByRole('button', { name: /编辑地址\/?令牌/ }).click()
  await syncInput.fill('http://changed.test')
  await page.locator('.home-card').filter({ hasText: '消息同步' }).getByRole('button', { name: '保存' }).click({ force: true })
  await expect(syncInput).toHaveValue('http://changed.test')
  await backupTextArea.fill(dataBackup)
  await page.getByRole('button', { name: '导入合并' }).click({ force: true })
  await expect(page.getByText(/完整数据备份已合并/)).toBeVisible()
  await expect(syncInput).toHaveValue('http://changed.test')
  await page.getByRole('button', { name: '导入覆盖' }).click({ force: true })
  await expect(page.getByText('完整数据备份已导入')).toBeVisible()
  await expect(syncInput).toHaveValue('http://sync.test')
  await expect(page.getByLabel('当前会话自动发送已读回执')).not.toBeChecked()
  await expect(page.getByText(/DHT 操作历史：.*备份测试/)).toBeVisible()
  await page.locator('summary').filter({ hasText: '导入 DHT 历史' }).click()
  await page.getByLabel('DHT 操作历史 JSON').fill(JSON.stringify({ dht: { operation_history: ['2026/07/16 10:20 · DHT 查找：导入测试', `2026/07/16 10:23 · DHT 查找：${'x'.repeat(320)}`, '2026/07/16 10:21 · DHT 查找：[已脱敏]', '2026/07/16 10:22 · DHT 查找：... [已截断]'] } }))
  await page.getByRole('button', { name: '导入 DHT 历史' }).click()
  await expect(page.getByRole('heading', { name: '导入 DHT 操作历史' })).toBeVisible()
  await expect(page.getByText(/将导入 2 条 DHT 操作历史/)).toBeVisible()
  await page.getByRole('button', { name: '确定' }).click()
  await expect(page.getByText(/DHT 操作历史：.*导入测试/)).toBeVisible()
  await expect(page.getByText(/已脱敏/)).not.toBeVisible()
})

test('消息合并会保留更高回执状态和时间戳', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'MergeReceipt', 'merge receipt 2026')
  const result = await page.evaluate(() => {
    const base = {
      id: 'local-msg',
      conversation_id: 'conv-peer',
      peer_user_id: 'peer',
      direction: 'out',
      text: 'hello',
      protocol_message_id: 'proto-1',
      status: 'mailbox',
      created_at: 1000,
    }
    const incoming = {
      id: 'remote-msg-different-local-id',
      conversation_id: 'conv-peer',
      peer_user_id: 'peer',
      direction: 'out',
      text: 'hello',
      protocol_message_id: 'proto-1',
      status: 'read',
      delivered_at: 2000,
      read_at: 3000,
      created_at: 1000,
    }
    return (window as any).mergeMessagesForTests([base], [incoming])
  })
  expect(result.added).toBe(0)
  expect(result.merged).toBe(1)
  expect(result.items).toHaveLength(1)
  expect(result.items[0].status).toBe('read')
  expect(result.items[0].delivered_at).toBe(2000)
  expect(result.items[0].read_at).toBe(3000)
})

test('删除本地身份会清理该身份的 IndexedDB 分表数据', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'DeleteMe', 'delete local identity 2026')
  await enableSync(page)
  const identity = await localIdentityRecord(page)
  await expect.poll(async () => await idbStoreEntryForUser(page, identity.user_id, 'meta', 'main')).not.toBeNull()
  await page.goto('/#/me')
  await page.getByRole('button', { name: '退出登录' }).click({ force: true })
  await page.getByRole('button', { name: '删除本地身份' }).click({ force: true })
  await expect(page.getByRole('heading', { name: '删除本地身份' })).toBeVisible()
  await page.getByRole('button', { name: '确定' }).click({ force: true })
  await expect(page.getByText('本机还没有保存的身份。')).toBeVisible()
  expect(await idbStoreEntryForUser(page, identity.user_id, 'meta', 'main')).toBeNull()
  expect(await idbStoreEntryForUser(page, identity.user_id, 'contacts', identity.user_id)).toBeNull()
})

test('重加密身份会轮换本地 IndexedDB 加密密钥并保留数据可登录', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'Rotate', 'old local storage key 2026')
  await enableSync(page)
  await page.goto('/#/chat')
  await page.getByPlaceholder('搜索聊天').fill('')
  await page.goto('/#/me')
  await fieldAfterLabel(page, '新提示词', 'input').fill('new local storage key 2026')
  await page.getByRole('button', { name: '重加密身份' }).click({ force: true })
  await expect(page.getByText('身份备份已重加密，请重新导出保存')).toBeVisible()
  const identity = await localIdentityRecord(page)
  expect(identity.backup_text).toContain('lm-identity-backup-v1:')
  const metaAfterRotate = await idbStoreEntry(page, 'meta', 'main')
  expect(metaAfterRotate.nodeControlUrl.__lm_enc_v1).toBe(true)
  expect(JSON.stringify(metaAfterRotate)).not.toContain('sync.test')
  await page.getByRole('button', { name: '退出登录' }).click({ force: true })
  await fieldAfterLabel(page, '提示词').fill('new local storage key 2026')
  await page.getByRole('button', { name: '登录', exact: true }).click({ force: true })
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.goto('/#/me')
  await expect(page.getByRole('button', { name: '关闭同步' })).toBeVisible()
  await expect(page.getByText('http://sync.test', { exact: true })).toBeVisible()
})

test('IndexedDB 单条损坏不会阻断其余本地数据恢复', async ({ page }) => {
  await clearBrowserState(page)
  await createIdentity(page, 'Recover', 'recover local state 2026')
  const identity = await localIdentityRecord(page)
  await page.evaluate(async () => { await (window as any).flushPersistForTests?.() })
  await idbPutRaw(page, 'messages', 'corrupt-message', {
    id: 'corrupt-message',
    conversation_id: `conv-${identity.user_id}`,
    peer_user_id: identity.user_id,
    direction: 'in',
    text: { __lm_enc_v1: true, alg: 'AES-GCM', kdf: 'PBKDF2-SHA-256', iv: 'not-base64', ct: 'not-base64' },
    status: 'received',
    created_at: Date.now(),
  })
  await page.reload()
  await fieldAfterLabel(page, '提示词').fill('recover local state 2026')
  await page.getByRole('button', { name: '登录', exact: true }).click({ force: true })
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.goto('/#/me')
  await expect(page.getByRole('heading', { name: 'Recover' })).toBeVisible()
  await expect(page.locator('.toast.warning')).toContainText('已跳过损坏的本地记录')
})

test('注册后可在独立导入页导入身份，再回登录页登录', async ({ page }) => {
  await clearBrowserState(page)
  await page.goto('/#/register')
  await expect(page.getByRole('heading', { name: '注册 LM Talk' })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '注册', exact: true }).click({ force: true })
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()

  const identityText = (await localIdentityRecord(page)).backup_text
  expect(identityText).toContain('lm-identity-backup-v1:')

  await clearBrowserState(page)
  await page.goto('/#/import')
  await expect(page.getByRole('heading', { name: '导入身份', level: 1 })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('wrong passphrase')
  await fieldAfterLabel(page, '身份文本').fill(identityText)
  await page.getByRole('button', { name: '导入', exact: true }).click({ force: true })
  await expect(page.getByText('提示词不正确，请重新输入。')).toBeVisible()
  await page.getByRole('button', { name: '知道了' }).click({ force: true })

  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '导入', exact: true }).click({ force: true })
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '登录', exact: true }).click({ force: true })
  await expect(page.locator('.chat-shell')).toBeVisible()
})
