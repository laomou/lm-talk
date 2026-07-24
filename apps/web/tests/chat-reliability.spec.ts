import { expect, test, type BrowserContext, type Page } from '@playwright/test'

const NODE_ENTRY = 'http://127.0.0.1:8787|playwright-node-token'

async function waitForWasm(page: Page) {
  await expect(page.getByText('正在准备')).toBeHidden({ timeout: 30_000 })
}

async function openMe(page: Page) {
  await page.locator('.rail-avatar[aria-label="打开我的设置"]').click()
}

async function registerAndLogin(context: BrowserContext, name: string, passphrase: string): Promise<Page> {
  const page = await context.newPage()
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册' }).click()
  await expect(page.getByRole('heading', { name: '身份已创建' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/#\/chat$/)

  await openMe(page)
  await page.getByText('同步与安全', { exact: true }).click()
  const address = page.getByLabel('同步服务地址')
  if (!await address.isVisible()) await page.getByRole('button', { name: '编辑地址' }).click()
  await address.fill(NODE_ENTRY)
  await page.getByRole('button', { name: '保存' }).click()
  await page.getByRole('button', { name: '开启同步' }).click()
  await expect(page.getByText('已开启', { exact: true }).first()).toBeVisible()
  await page.getByRole('button', { name: '返回我' }).click()
  await page.getByText('个人资料', { exact: true }).click()
  await page.getByLabel('显示名').fill(name)
  await page.getByRole('button', { name: '保存' }).click()
  await page.getByRole('button', { name: '返回我' }).click()
  return page
}

async function copyOwnCard(page: Page): Promise<string> {
  await openMe(page)
  await page.getByText('个人资料', { exact: true }).click()
  await page.getByRole('button', { name: '我的名片' }).click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toBeVisible()
  await dialog.getByRole('button', { name: '复制原文' }).click()
  const card = await page.evaluate(() => navigator.clipboard.readText())
  await dialog.getByRole('button', { name: '关闭', exact: true }).click()
  await page.getByRole('button', { name: '返回我' }).click()
  return card
}

async function openOnlyContactConversation(page: Page) {
  await page.getByRole('button', { name: '打开通讯录' }).click()
  const contact = page.locator('.directory-row.contact-row').first()
  await expect(contact).toBeVisible()
  await contact.click()
  await page.getByRole('button', { name: '发消息' }).click()
}

async function reloadAndLogin(page: Page, passphrase: string) {
  const expectedPath = new URL(page.url()).hash
  await page.reload()
  await waitForWasm(page)
  await expect(page.getByRole('heading', { name: '登录' })).toBeVisible()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(new RegExp(`${expectedPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`))
}

async function flushLocalPersistence(page: Page) {
  await page.evaluate(async () => {
    await (window as typeof window & { flushPersistForTests?: () => Promise<void> }).flushPersistForTests?.()
  })
}

async function openSyncSettings(page: Page) {
  await openMe(page)
  await page.getByText('同步与安全', { exact: true }).click()
}

async function persistedTableCount(page: Page, table: string): Promise<number> {
  return page.evaluate(async (tableName) => {
    const request = indexedDB.open('lm-talk-web-db')
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
    const tx = db.transaction(tableName, 'readonly')
    const countRequest = tx.objectStore(tableName).count()
    const count = await new Promise<number>((resolve, reject) => {
      countRequest.onsuccess = () => resolve(countRequest.result)
      countRequest.onerror = () => reject(countRequest.error)
    })
    db.close()
    return count
  }, table)
}

async function mailboxDeliveryTotal(page: Page, userId: string): Promise<number> {
  return page.evaluate(async ({ nodeEntry, mailboxUserId }) => {
    const [baseUrl, token] = nodeEntry.split('|')
    const response = await fetch(`${baseUrl}/api/mailbox/status?user_id=${encodeURIComponent(mailboxUserId)}`, {
      headers: token ? { authorization: `Bearer ${token}` } : undefined,
    })
    if (!response.ok) throw new Error(`mailbox status returned ${response.status}`)
    const body = await response.json()
    return Number(body.summary?.total ?? -1)
  }, { nodeEntry: NODE_ENTRY, mailboxUserId: userId })
}

test('双用户批量消息在刷新重连后保持顺序、去重、未读与已读回执', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-alice-passphrase'
  const bobPassphrase = 'playwright-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)

  try {
    const bobCard = await copyOwnCard(bob)
    await alice.getByRole('button', { name: '打开通讯录' }).click()
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByLabel('对方名片').fill(bobCard)
    await alice.getByRole('button', { name: '添加好友' }).click()
    await expect(alice.getByLabel('对方名片')).toHaveValue('')
    await alice.getByRole('button', { name: '返回通讯录' }).click()

    await bob.getByRole('button', { name: '打开通讯录' }).click()
    await bob.getByRole('button', { name: '打开新的朋友' }).click()
    await expect(bob.getByRole('button', { name: '同意' })).toBeVisible({ timeout: 45_000 })
    await bob.getByRole('button', { name: '同意' }).click()
    await bob.getByRole('button', { name: '返回通讯录' }).click()
    await bob.locator('.directory-row.contact-row').click()
    await bob.getByRole('button', { name: '开启已读回执' }).click()
    await expect(bob.getByRole('button', { name: '已开启已读回执' })).toBeVisible()
    await bob.getByRole('button', { name: '发消息' }).click()

    // Mailbox long-poll completes the friend response and the secure-session handshake.
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    // Wait for the app's normal IndexedDB persistence pipeline to settle before
    // the destructive browser lifecycle event. This does not seed or alter
    // state; it only makes the real write completion observable to the test.
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'contacts')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'contacts')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await reloadAndLogin(alice, alicePassphrase)
    await reloadAndLogin(bob, bobPassphrase)

    // Compose while offline, then refresh and log in again. The durable outbox
    // must survive and resume automatically when sync is restored.
    await openSyncSettings(alice)
    await alice.getByRole('button', { name: '关闭同步' }).click()
    await expect(alice.getByRole('button', { name: '开启同步' })).toBeVisible()
    await alice.getByRole('button', { name: '返回我' }).click()
    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    const queuedMessages = ['断线批量第一条', '断线批量第二条', '🎉', '断线批量第四条']
    for (const text of queuedMessages) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
      await expect(aliceMessages.getByText(text, { exact: true })).toBeVisible()
    }
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(queuedMessages.length)
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox')).toBe(queuedMessages.length)
    await reloadAndLogin(alice, alicePassphrase)
    await openSyncSettings(alice)
    await alice.getByRole('button', { name: '开启同步' }).click()
    await expect(alice.getByRole('button', { name: '关闭同步' })).toBeVisible()
    await alice.getByRole('button', { name: '返回我' }).click()

    // Bob stays on the conversation list. The badge proves batch delivery does
    // not force-switch the current view and that unread state is retained.
    await expect(bob.locator('.conversation-badge')).toHaveText(String(queuedMessages.length), { timeout: 45_000 })
    await reloadAndLogin(bob, bobPassphrase)
    await expect(bob.locator('.conversation-badge')).toHaveText(String(queuedMessages.length))

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(queuedMessages, { timeout: 45_000 })
    for (const text of queuedMessages) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    // A read receipt supersedes the visible "delivered" label. Bob receiving the
    // message plus Alice receiving the final read status covers the full flow.
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(queuedMessages.length).fill('已读'),
      { timeout: 45_000 },
    )

    await bob.getByLabel('输入消息').fill('🎉')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(bobMessages.locator('.bubble.out .text').getByText('🎉', { exact: true })).toHaveCount(1)

    await openOnlyContactConversation(alice)
    await expect(
      alice.getByRole('log', { name: '消息列表' }).locator('.bubble.in .text').getByText('🎉', { exact: true }),
    ).toHaveCount(1, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('节点暂不可用后自动恢复批量消息、未读与已读状态', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-network-alice-passphrase'
  const bobPassphrase = 'playwright-network-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)

  try {
    const bobCard = await copyOwnCard(bob)
    await alice.getByRole('button', { name: '打开通讯录' }).click()
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByLabel('对方名片').fill(bobCard)
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByRole('button', { name: '返回通讯录' }).click()

    await bob.getByRole('button', { name: '打开通讯录' }).click()
    await bob.getByRole('button', { name: '打开新的朋友' }).click()
    await expect(bob.getByRole('button', { name: '同意' })).toBeVisible({ timeout: 45_000 })
    await bob.getByRole('button', { name: '同意' }).click()
    await bob.getByRole('button', { name: '返回通讯录' }).click()
    await bob.locator('.directory-row.contact-row').click()
    await bob.getByRole('button', { name: '开启已读回执' }).click()
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    // Finish the real friendship/session handshake before testing transport
    // failure. This keeps the scenario focused on a durable chat Outbox rather
    // than setup-time friend and secure-session control payloads.
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await reloadAndLogin(alice, alicePassphrase)
    await reloadAndLogin(bob, bobPassphrase)

    // Keep Bob out of the conversation so incoming messages remain unread until
    // the user intentionally opens the chat.
    await bob.locator('.rail-avatar[aria-label="打开我的设置"]').click()
    await openOnlyContactConversation(alice)
    // Simulate a connection refusal only on Alice's transport to the real
    // lm_node. No mailbox response, IndexedDB data, Ratchet state, or message
    // payload is mocked; the node remains running and Bob stays connected.
    await aliceContext.route('http://127.0.0.1:8787/api/**', (route) => route.abort('connectionrefused'))
    const queuedMessages = ['网络恢复第一条', '网络恢复第二条', '🚀']
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    for (const text of queuedMessages) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
      await expect(aliceMessages.getByText(text, { exact: true })).toBeVisible()
    }
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox'), { timeout: 45_000 }).toBe(queuedMessages.length)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(queuedMessages.length)

    // Restore access to the actual lm_node and emit the browser recovery
    // event. No mailbox, IndexedDB, Ratchet, or message state is mocked.
    await aliceContext.unroute('http://127.0.0.1:8787/api/**')
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))

    await expect(bob.locator('.rail-badge')).toHaveText(String(queuedMessages.length), { timeout: 45_000 })
    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(queuedMessages, { timeout: 45_000 })
    for (const text of queuedMessages) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.rail-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(queuedMessages.length).fill('已读'),
      { timeout: 45_000 },
    )
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('接收端 ACK 中断后自动去重并清空 Mailbox', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-ack-alice-passphrase'
  const bobPassphrase = 'playwright-ack-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let ackAttempts = 0

  bobContext.on('request', (request) => {
    const url = new URL(request.url())
    if (url.pathname === '/api/mailbox/take') {
      bobUserId = url.searchParams.get('user_id') || bobUserId
    }
  })

  try {
    const bobCard = await copyOwnCard(bob)
    await alice.getByRole('button', { name: '打开通讯录' }).click()
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByLabel('对方名片').fill(bobCard)
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByRole('button', { name: '返回通讯录' }).click()

    await bob.getByRole('button', { name: '打开通讯录' }).click()
    await bob.getByRole('button', { name: '打开新的朋友' }).click()
    await expect(bob.getByRole('button', { name: '同意' })).toBeVisible({ timeout: 45_000 })
    await bob.getByRole('button', { name: '同意' }).click()
    await bob.getByRole('button', { name: '返回通讯录' }).click()
    await bob.locator('.directory-row.contact-row').click()
    await bob.getByRole('button', { name: '开启已读回执' }).click()
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await reloadAndLogin(alice, alicePassphrase)
    await reloadAndLogin(bob, bobPassphrase)
    await expect.poll(() => bobUserId).not.toBe('')
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    // Keep the receiver away from the conversation so the recovered messages
    // remain unread. Keep ACK unavailable until the refresh: the next real
    // mailbox take after login must receive duplicate deliveries, dedupe
    // locally, then acknowledge them.
    await bob.locator('.rail-avatar[aria-label="打开我的设置"]').click()
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/ack', async (route) => {
      ackAttempts += 1
      await route.abort('connectionrefused')
    })

    await openOnlyContactConversation(alice)
    const texts = ['ACK 恢复第一条', '📬']
    for (const text of texts) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
    }

    await expect.poll(() => ackAttempts, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(texts.length)
    await expect(bob.locator('.rail-badge')).toHaveText(String(texts.length), { timeout: 45_000 })
    // Reload after the ACK failure. The received messages and dedupe records
    // must survive locally; the next real mailbox take sees the same delivery,
    // skips duplicate rendering, and sends the replacement ACK.
    await bobContext.unroute('http://127.0.0.1:8787/api/mailbox/ack')
    await reloadAndLogin(bob, bobPassphrase)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(texts, { timeout: 45_000 })
    for (const text of texts) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(texts.length).fill('已读'),
      { timeout: 45_000 },
    )
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('接收端长轮询中断后无需刷新即可恢复收取消息', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-long-poll-alice-passphrase'
  const bobPassphrase = 'playwright-long-poll-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let interruptedLongPolls = 0

  bobContext.on('request', (request) => {
    const url = new URL(request.url())
    if (url.pathname === '/api/mailbox/take') {
      bobUserId = url.searchParams.get('user_id') || bobUserId
    }
  })

  try {
    const bobCard = await copyOwnCard(bob)
    await alice.getByRole('button', { name: '打开通讯录' }).click()
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByLabel('对方名片').fill(bobCard)
    await alice.getByRole('button', { name: '添加好友' }).click()
    await alice.getByRole('button', { name: '返回通讯录' }).click()

    await bob.getByRole('button', { name: '打开通讯录' }).click()
    await bob.getByRole('button', { name: '打开新的朋友' }).click()
    await expect(bob.getByRole('button', { name: '同意' })).toBeVisible({ timeout: 45_000 })
    await bob.getByRole('button', { name: '同意' }).click()
    await bob.getByRole('button', { name: '返回通讯录' }).click()
    await bob.locator('.directory-row.contact-row').click()
    await bob.getByRole('button', { name: '开启已读回执' }).click()
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await reloadAndLogin(alice, alicePassphrase)
    await reloadAndLogin(bob, bobPassphrase)
    await expect.poll(() => bobUserId).not.toBe('')
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    // Interrupt a real auto-poll request rather than faking a mailbox response.
    // The route remains active until recovery so Bob cannot consume Alice's
    // messages through another request before the long-poll retry starts.
    await bob.locator('.rail-avatar[aria-label="打开我的设置"]').click()
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/take**', async (route) => {
      const url = new URL(route.request().url())
      if (url.searchParams.has('wait_seconds')) interruptedLongPolls += 1
      await route.abort('connectionrefused')
    })
    await expect.poll(() => interruptedLongPolls, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await expect(bob).toHaveURL(/#\/me$/)

    await openOnlyContactConversation(alice)
    const texts = ['长轮询恢复第一条', '📨']
    for (const text of texts) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
    }
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(texts.length)

    // Do not reload, re-login, or fire an online event. Restoring the real
    // node transport must be enough for the long-poll backoff loop to recover.
    await bobContext.unroute('http://127.0.0.1:8787/api/mailbox/take**')
    await expect(bob.locator('.rail-badge')).toHaveText(String(texts.length), { timeout: 45_000 })
    await expect(bob).toHaveURL(/#\/me$/)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(texts, { timeout: 45_000 })
    for (const text of texts) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(texts.length).fill('已读'),
      { timeout: 45_000 },
    )
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})
