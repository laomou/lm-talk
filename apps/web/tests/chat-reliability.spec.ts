import { expect, test, type BrowserContext, type Page } from '@playwright/test'

const NODE_ENTRY = 'http://127.0.0.1:8787|playwright-node-token'

async function waitForWasm(page: Page) {
  await expect(page.getByText('正在准备')).toBeHidden({ timeout: 30_000 })
}

async function openMe(page: Page) {
  await page.locator('.app-rail .rail-item[aria-label="打开我的设置"]').click()
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
  await page.locator('.rail-avatar[aria-label="打开我的设置"]').click()
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
  const rows = page.locator('.directory-row.contact-row')
  await expect.poll(async () => {
    await page.getByRole('button', { name: '打开通讯录' }).click().catch(() => undefined)
    let count = await rows.count()
    if (count === 0) {
      await takeMailbox(page).catch(() => undefined)
      await page.getByRole('button', { name: '打开通讯录' }).click().catch(() => undefined)
      count = await rows.count()
    }
    return count
  }, { timeout: 90_000 }).toBeGreaterThan(0)
  await rows.first().click()
  await page.getByRole('button', { name: '发消息' }).click()
}

async function reloadAndLogin(page: Page, passphrase: string) {
  const expectedPath = new URL(page.url()).hash
  await page.reload()
  await waitForWasm(page)
  await expect(page.getByRole('heading', { name: '登录' })).toBeVisible()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page.locator('.app-rail')).toBeVisible({ timeout: 45_000 })
  await expect(page).toHaveURL(new RegExp(`${expectedPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`))
}

async function flushLocalPersistence(page: Page) {
  await page.evaluate(async () => {
    await (window as typeof window & { flushPersistForTests?: () => Promise<void> }).flushPersistForTests?.()
  })
}

async function takeMailbox(page: Page) {
  await page.evaluate(async () => {
    await (window as typeof window & { takeMailboxForTests?: () => Promise<void> }).takeMailboxForTests?.()
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
    await bob.getByRole('button', { name: '返回聊天列表' }).click()

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

test('保存昵称后会自动同步 Contact Card 到好友', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-profile-sync-alice-passphrase'
  const bobPassphrase = 'playwright-profile-sync-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)

    await openMe(alice)
    await alice.getByText('个人资料', { exact: true }).click()
    await alice.getByLabel('显示名').fill('Alice 更新后昵称')
    const contactUpdatePush = alice.waitForResponse((response) =>
      response.url().includes('/api/mailbox/push') && response.request().method() === 'POST',
    )
    await alice.getByRole('button', { name: '保存' }).click()
    await expect((await contactUpdatePush).ok()).toBeTruthy()
    await takeMailbox(bob)
    await bob.getByRole('button', { name: '打开通讯录' }).click()
    await expect(bob.locator('.directory-row.contact-row').getByText('Alice 更新后昵称', { exact: true }))
      .toBeVisible({ timeout: 45_000 })
    await bob.locator('.directory-row.contact-row').click()
    await expect(bob.getByRole('heading', { name: 'Alice 更新后昵称' })).toBeVisible()
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('保存头像后会自动同步到好友列表和聊天页', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-avatar-sync-alice-passphrase'
  const bobPassphrase = 'playwright-avatar-sync-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)

    await openMe(alice)
    await alice.getByText('个人资料', { exact: true }).click()
    const avatar = Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="32" height="32" fill="#4c8df6"/><circle cx="16" cy="16" r="9" fill="#fff"/></svg>',
    )
    const avatarUpdatePush = alice.waitForResponse((response) =>
      response.url().includes('/api/mailbox/push') && response.request().method() === 'POST',
    )
    await alice.setInputFiles('#avatar-input', { name: 'avatar.svg', mimeType: 'image/svg+xml', buffer: avatar })
    await expect((await avatarUpdatePush).ok()).toBeTruthy()

    await takeMailbox(bob)
    await bob.getByRole('button', { name: '打开通讯录' }).click()
    const contactRow = bob.locator('.directory-row.contact-row').filter({ hasText: 'Alice' }).first()
    await expect(contactRow.locator('.avatar img')).toBeVisible({ timeout: 45_000 })
    await contactRow.click()
    await expect(bob.locator('.detail-hero .avatar img')).toBeVisible()
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(bob.locator('.chat-header .avatar img')).toBeVisible()
    await bob.getByLabel('输入消息').fill('头像同步后创建会话')
    await bob.getByRole('button', { name: '发送' }).click()
    await bob.getByRole('button', { name: '返回聊天列表' }).click()
    await expect(bob.locator('.conversation-list .avatar img')).toBeVisible()
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('单聊附件在消息卡片中解密，刷新后可重新解密', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-attachment-alice-passphrase'
  const bobPassphrase = 'playwright-attachment-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)

    await openOnlyContactConversation(alice)
    await alice.getByLabel('选择附件').setInputFiles({
      name: 'hello.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('lm-talk encrypted attachment'),
    })
    await alice.getByRole('button', { name: '发送文件' }).click()
    await expect(alice.getByRole('log', { name: '消息列表' }).getByText('hello.txt', { exact: true })).toBeVisible()

    await expect(bob.getByRole('log', { name: '消息列表' }).getByText('hello.txt', { exact: true }))
      .toBeVisible({ timeout: 45_000 })
    const bobAttachment = bob.locator('.bubble.in .message-attachment-card').filter({ hasText: 'hello.txt' })
    await bobAttachment.getByRole('button', { name: '解密文件' }).click()
    await expect(bobAttachment.getByRole('link', { name: '下载' })).toBeVisible({ timeout: 45_000 })

    await flushLocalPersistence(bob)
    await reloadAndLogin(bob, bobPassphrase)
    const restoredAttachment = bob.locator('.bubble.in .message-attachment-card').filter({ hasText: 'hello.txt' })
    await expect(restoredAttachment.getByText('页面刷新后需要重新解密，才能预览或下载。')).toBeVisible()
    await expect(restoredAttachment.getByRole('button', { name: '重试解密' })).toBeVisible()

    await bob.getByRole('button', { name: '更多' }).click()
    await bob.getByRole('menuitem', { name: '删除会话' }).click()
    await bob.getByRole('dialog').getByRole('button', { name: '确定' }).click()
    await expect(bob).toHaveURL(/#\/chat$/)
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('会话内搜索命中旧消息时会自动展开历史窗口并定位', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-search-window-alice-passphrase'
  const bobPassphrase = 'playwright-search-window-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await openOnlyContactConversation(alice)

    const batch = Array.from({ length: 90 }, (_, index) => `历史窗口消息 ${index + 1}`)
    for (const text of batch) {
      await bob.getByLabel('输入消息').fill(text)
      await bob.getByRole('button', { name: '发送' }).click()
    }

    await alice.getByRole('button', { name: '搜索聊天记录' }).click()
    await alice.getByLabel('搜索聊天记录').fill('历史窗口消息')
    const firstSearchResult = alice.locator('.search-message-result').filter({ hasText: '历史窗口消息 1' }).first()
    await expect(firstSearchResult).toBeVisible({ timeout: 45_000 })
    await firstSearchResult.click()
    await expect(alice.getByRole('log', { name: '消息列表' }).getByText(/历史窗口消息 1/))
      .toBeVisible({ timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('长会话默认窗口渲染并可加载更早消息', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-history-window-alice-passphrase'
  const bobPassphrase = 'playwright-history-window-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)

    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    const messages = Array.from({ length: 90 }, (_, index) => `长历史消息 ${String(index + 1).padStart(3, '0')}`)
    for (const text of messages) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
      await expect(aliceMessages.getByText(text, { exact: true })).toBeVisible()
    }

    await expect(aliceMessages.getByText('长历史消息 090', { exact: true })).toBeVisible()
    await expect(aliceMessages.getByText('长历史消息 001', { exact: true })).toHaveCount(0)
    await expect(alice.getByRole('button', { name: '加载更早的 10 条消息' })).toBeVisible()
    await alice.getByRole('button', { name: '加载更早的 10 条消息' }).click()
    await expect(aliceMessages.getByText('长历史消息 001', { exact: true })).toBeVisible()
    await expect(alice.getByRole('button', { name: /加载更早/ })).toHaveCount(0)
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('聊天消息操作统一到同一菜单入口', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-message-menu-alice-passphrase'
  const bobPassphrase = 'playwright-message-menu-bob-passphrase'
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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await openOnlyContactConversation(alice)

    await alice.getByLabel('输入消息').fill('菜单里的这条消息')
    await alice.getByRole('button', { name: '发送' }).click()
    const bubble = alice.locator('.bubble.out').last()
    await expect(bubble.getByText('菜单里的这条消息', { exact: true })).toBeVisible()
    await bubble.getByRole('button', { name: '消息操作' }).click()
    await expect(alice.getByRole('menuitem', { name: '复制文本' })).toBeVisible()
    await expect(alice.getByRole('menuitem', { name: '复制密文' })).toBeVisible()
    await expect(alice.getByRole('menuitem', { name: '删除消息' })).toBeVisible()
    await alice.getByRole('menuitem', { name: '删除消息' }).click()
    await expect(alice.getByRole('dialog')).toBeVisible({ timeout: 45_000 })
    await alice.getByRole('dialog').getByRole('button', { name: '确定' }).click()
    await expect(bubble.getByText('菜单里的这条消息', { exact: true })).toHaveCount(0, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('连续持久化请求会被合并成少量写入', async ({ browser }) => {
  const context = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const page = await registerAndLogin(context, 'Alice', 'playwright-persist-merge-passphrase')

  try {
    const before = await page.evaluate(() => (window as typeof window & { getPersistMetricsForTests?: () => { flushes: number } }).getPersistMetricsForTests?.()?.flushes ?? 0)
    await page.evaluate(async () => {
      const win = window as typeof window & { setDhtDiagnosticsForTests?: (status: string, history?: string[]) => void; flushPersistForTests?: () => Promise<void> }
      for (let i = 0; i < 12; i += 1) {
        win.setDhtDiagnosticsForTests?.(`persist-burst-${i}`, [`item-${i}`])
      }
      await win.flushPersistForTests?.()
    })
    const after = await page.evaluate(() => (window as typeof window & { getPersistMetricsForTests?: () => { flushes: number } }).getPersistMetricsForTests?.()?.flushes ?? 0)
    expect(after - before).toBeLessThanOrEqual(3)
  } finally {
    await context.close()
  }
})

test('附件发送失败后可重试并最终送达', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-attachment-send-retry-alice-passphrase'
  const bobPassphrase = 'playwright-attachment-send-retry-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let restoreAliceTransport = false

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
    await bob.getByRole('button', { name: '发消息' }).click()
    await expect(alice.locator('.directory-row.contact-row')).toBeVisible({ timeout: 45_000 })
    await openOnlyContactConversation(alice)
    await alice.evaluate(() => (window as typeof window & { resetRtcForTests?: () => void }).resetRtcForTests?.())
    await aliceContext.route('http://127.0.0.1:8787/api/**', async (route) => {
      if (restoreAliceTransport) {
        await route.continue()
        return
      }
      await route.abort('connectionrefused')
    })

    await alice.getByLabel('选择附件').setInputFiles({
      name: 'retry.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('attachment retry payload'),
    })
    await alice.getByRole('button', { name: '发送文件' }).click()
    await expect(alice.getByRole('button', { name: '重试发送' })).toBeVisible({ timeout: 45_000 })

    restoreAliceTransport = true
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))
    await alice.getByRole('button', { name: '重试发送' }).click()
    const bobAttachment = bob.locator('.bubble.in .message-attachment-card').filter({ hasText: 'retry.txt' }).first()
    await expect(bobAttachment).toBeVisible({ timeout: 45_000 })
    await bobAttachment.getByRole('button', { name: '解密文件' }).click()
    await expect(bobAttachment.getByRole('link', { name: '下载' })).toBeVisible({ timeout: 45_000 })
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
  let restoreAliceTransport = false

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
    await openMe(bob)
    await openOnlyContactConversation(alice)
    // Simulate a connection refusal only on Alice's transport to the real
    // lm_node. No mailbox response, IndexedDB data, Ratchet state, or message
    // payload is mocked; the node remains running and Bob stays connected.
    await aliceContext.route('http://127.0.0.1:8787/api/**', async (route) => {
      if (restoreAliceTransport) {
        await route.continue()
        return
      }
      await route.abort('connectionrefused')
    })
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
    restoreAliceTransport = true
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(queuedMessages, { timeout: 45_000 })
    for (const text of queuedMessages) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
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
    await openMe(bob)
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
    // Reload after the ACK failure. The next real mailbox take sees the same
    // deliveries, dedupes any local state already written, and sends replacement ACKs.
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
  let restoreBobTransport = false

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
    // Keep the route installed and restore its real transport in-place: removing
    // a context route while a long-poll retry is being scheduled has proved
    // timing-sensitive in CI.
    await openMe(bob)
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/take**', async (route) => {
      const url = new URL(route.request().url())
      if (restoreBobTransport) {
        await route.continue()
        return
      }
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
    restoreBobTransport = true
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

test('双向并发消息在短暂断网恢复后保持 Ratchet 顺序与回执一致', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-bidirectional-alice-passphrase'
  const bobPassphrase = 'playwright-bidirectional-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let aliceUserId = ''
  let bobUserId = ''
  let interruptedAliceRequests = 0

  aliceContext.on('request', (request) => {
    const url = new URL(request.url())
    if (url.pathname === '/api/mailbox/take') {
      aliceUserId = url.searchParams.get('user_id') || aliceUserId
    }
  })
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
    await alice.locator('.directory-row.contact-row').click()
    await alice.getByRole('button', { name: '开启已读回执' }).click()
    await expect(alice.getByRole('button', { name: '已开启已读回执' })).toBeVisible()
    await alice.getByRole('button', { name: '发消息' }).click()
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await reloadAndLogin(alice, alicePassphrase)
    await reloadAndLogin(bob, bobPassphrase)
    await expect.poll(() => aliceUserId).not.toBe('')
    await expect.poll(() => bobUserId).not.toBe('')
    await expect.poll(() => mailboxDeliveryTotal(alice, aliceUserId), { timeout: 45_000 }).toBe(0)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(alice)
    // Keep Bob outside the conversation so Alice's recovered batch is observed
    // as unread. Wait until Alice has a genuinely interrupted long-poll before
    // sending, rather than merely blocking a future request.
    await openMe(bob)
    await aliceContext.route('http://127.0.0.1:8787/api/**', async (route) => {
      interruptedAliceRequests += 1
      await route.abort('connectionrefused')
    })
    await expect.poll(() => interruptedAliceRequests, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)

    const aliceTexts = ['Alice 并发第一条', '⚡', 'Alice 并发第三条']
    const bobTexts = ['Bob 并发第一条', '🛰️', 'Bob 并发第三条']
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await Promise.all([
      (async () => {
        for (const text of aliceTexts) {
          await alice.getByLabel('输入消息').fill(text)
          await alice.getByRole('button', { name: '发送' }).click()
          await expect(aliceMessages.getByText(text, { exact: true })).toBeVisible()
        }
      })(),
      (async () => {
        await openOnlyContactConversation(bob)
        for (const text of bobTexts) {
          await bob.getByLabel('输入消息').fill(text)
          await bob.getByRole('button', { name: '发送' }).click()
        }
        await openMe(bob)
      })(),
    ])
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox'), { timeout: 45_000 }).toBe(aliceTexts.length)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(aliceTexts.length)

    // Restore only the actual node transport and emit the ordinary browser
    // recovery signal. Both sides must converge without reload or re-login.
    await aliceContext.unroute('http://127.0.0.1:8787/api/**')
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))
    await expect(bob.locator('.rail-badge')).toHaveText(String(aliceTexts.length), { timeout: 45_000 })
    await expect(alice).toHaveURL(/#\/chat/)
    await expect(bob).toHaveURL(/#\/me$/)
    await expect.poll(() => mailboxDeliveryTotal(alice, aliceUserId), { timeout: 45_000 }).toBe(0)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.in .text')).toHaveText(
      bobTexts,
      { timeout: 45_000 },
    )
    for (const text of bobTexts) {
      await expect(alice.getByRole('log', { name: '消息列表' }).getByText(text, { exact: true })).toHaveCount(1)
    }

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(aliceTexts, { timeout: 45_000 })
    for (const text of aliceTexts) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(aliceTexts.length).fill('已读'),
      { timeout: 45_000 },
    )
    await expect(bob.getByRole('log', { name: '消息列表' }).locator('.bubble.out .message-status')).toHaveText(
      Array(bobTexts.length).fill('已读'),
      { timeout: 45_000 },
    )
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('节点已接收但发送响应丢失后重试不会重复解密或显示消息', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-unknown-outcome-alice-passphrase'
  const bobPassphrase = 'playwright-unknown-outcome-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let blockedBobTakes = 0
  let lostPushResponses = 0

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

    // Hold Bob's real mailbox take requests so we can observe both node
    // deliveries. The response to Alice's first push is lost only after
    // route.fetch() has let the real lm_node accept it.
    await openMe(bob)
    await expect(bob).toHaveURL(/#\/me$/)
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/take**', async (route) => {
      blockedBobTakes += 1
      await route.abort('connectionrefused')
    })
    await expect.poll(() => blockedBobTakes, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await aliceContext.route('http://127.0.0.1:8787/api/mailbox/push', async (route) => {
      lostPushResponses += 1
      await route.fetch()
      await route.abort('connectionrefused')
    })

    await openOnlyContactConversation(alice)
    const text = '节点已收但响应丢失'
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await alice.getByLabel('输入消息').fill(text)
    await alice.getByRole('button', { name: '发送' }).click()
    await expect.poll(() => lostPushResponses, { timeout: 45_000 }).toBe(1)
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox'), { timeout: 45_000 }).toBe(1)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(1)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(1)

    // The normal outbox retry creates a fresh outer MailboxMessage. This makes
    // the node legitimately hold two deliveries for the same Ratchet envelope.
    // The receiver must dedupe by the inner protocol message id.
    await aliceContext.unroute('http://127.0.0.1:8787/api/mailbox/push')
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(2)

    await bobContext.unroute('http://127.0.0.1:8787/api/mailbox/take**')
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(0)
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(['已读'], { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('节点已收但发送端立即刷新后可恢复未知投递结果', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-unknown-refresh-alice-passphrase'
  const bobPassphrase = 'playwright-unknown-refresh-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let blockedBobTakes = 0
  let lostPushResponses = 0
  let restoreAlicePushTransport = false
  let restoreBobTakeTransport = false

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

    // The node accepts the first encrypted envelope, then Alice loses the
    // response and immediately refreshes. Neither mailbox data nor IndexedDB
    // is mocked: only the HTTP response and Bob's receive transport are held.
    await openMe(bob)
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/take**', async (route) => {
      if (restoreBobTakeTransport) {
        await route.continue()
        return
      }
      blockedBobTakes += 1
      await route.abort('connectionrefused')
    })
    await expect.poll(() => blockedBobTakes, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await aliceContext.route('http://127.0.0.1:8787/api/mailbox/push', async (route) => {
      if (restoreAlicePushTransport) {
        await route.continue()
        return
      }
      lostPushResponses += 1
      await route.fetch()
      await route.abort('connectionrefused')
    })

    await openOnlyContactConversation(alice)
    const text = '未知投递结果后刷新恢复'
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await alice.getByLabel('输入消息').fill(text)
    await alice.getByRole('button', { name: '发送' }).click()
    await expect.poll(() => lostPushResponses, { timeout: 45_000 }).toBe(1)
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox')).toBe(1)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(1)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(1)

    await reloadAndLogin(alice, alicePassphrase)
    restoreAlicePushTransport = true
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))
    // The persisted Outbox retries its exact inner Ratchet envelope. lm_node
    // legitimately contains two outer deliveries, while Bob renders it once.
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBeGreaterThanOrEqual(2)
    restoreBobTakeTransport = true
    await takeMailbox(bob)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(bob, 'contacts'), { timeout: 45_000 }).toBeGreaterThan(0)
    await reloadAndLogin(bob, bobPassphrase)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(0)
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(['已读'], { timeout: 45_000 })
    await bob.getByLabel('输入消息').fill('未知结果恢复后继续收发')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('未知结果恢复后继续收发', { exact: true })).toHaveCount(1, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('Mailbox 积压超过单页时可分页恢复全部 Ratchet 消息', async ({ browser }) => {
  test.setTimeout(240_000)
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-pagination-alice-passphrase'
  const bobPassphrase = 'playwright-pagination-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let blockedBobTakes = 0
  let restoreBobTakeTransport = false
  let immediatePageTakesAfterRestore = 0

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

    // Keep the receiver offline at the mailbox transport layer. Alice still
    // sends normally through the real node, producing 51 encrypted records:
    // one more than the web client's real 50-message page size.
    await openMe(bob)
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/take**', async (route) => {
      const url = new URL(route.request().url())
      if (restoreBobTakeTransport) {
        if (!url.searchParams.has('wait_seconds')) immediatePageTakesAfterRestore += 1
        await route.continue()
        return
      }
      blockedBobTakes += 1
      await route.abort('connectionrefused')
    })
    await expect.poll(() => blockedBobTakes, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)

    await openOnlyContactConversation(alice)
    const batch = Array.from({ length: 51 }, (_, index) => `分页积压消息 ${String(index + 1).padStart(2, '0')}`)
    for (const text of batch) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
    }
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 90_000 }).toBe(batch.length)

    // A normal mailbox take returns the first 50 with `more: true`; the web
    // client must ACK that page and issue a second zero-wait take for item 51.
    restoreBobTakeTransport = true
    await expect.poll(() => immediatePageTakesAfterRestore, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 90_000 }).toBe(0)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(batch, { timeout: 90_000 })
    for (const text of batch) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(
      Array(batch.length).fill('已读'),
      { timeout: 90_000 },
    )
    await bob.getByLabel('输入消息').fill('分页恢复后继续收发')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('分页恢复后继续收发', { exact: true })).toHaveCount(1, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('发送端在加密投递失败后刷新可按顺序恢复 Ratchet 消息', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-send-refresh-alice-passphrase'
  const bobPassphrase = 'playwright-send-refresh-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let restoreAliceTransport = false

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

    await openMe(bob)
    await expect(bob).toHaveURL(/#\/me$/)
    await openOnlyContactConversation(alice)
    await aliceContext.route('http://127.0.0.1:8787/api/**', async (route) => {
      if (restoreAliceTransport) {
        await route.continue()
        return
      }
      await route.abort('connectionrefused')
    })
    const pending = ['刷新恢复第一条', '🧭', '刷新恢复第三条']
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    for (const text of pending) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
    }
    await expect(aliceMessages.getByText('待发送', { exact: true })).toHaveCount(pending.length, { timeout: 45_000 })
    // The queued message envelopes and advanced Ratchet state must be durable
    // together before the destructive browser lifecycle transition.
    await flushLocalPersistence(alice)
    await expect.poll(() => persistedTableCount(alice, 'outbox')).toBe(pending.length)
    await reloadAndLogin(alice, alicePassphrase)
    restoreAliceTransport = true
    await alice.evaluate(() => window.dispatchEvent(new Event('online')))

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(pending, { timeout: 45_000 })
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(
      Array(pending.length).fill('已读'),
      { timeout: 45_000 },
    )
    await bob.getByLabel('输入消息').fill('刷新后继续收发')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('刷新后继续收发', { exact: true })).toHaveCount(1, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('接收端在批量解密后刷新可恢复未确认消息、顺序与 Ratchet 会话', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-receive-refresh-alice-passphrase'
  const bobPassphrase = 'playwright-receive-refresh-bob-passphrase'
  const alice = await registerAndLogin(aliceContext, 'Alice', alicePassphrase)
  const bob = await registerAndLogin(bobContext, 'Bob', bobPassphrase)
  let bobUserId = ''
  let blockedAcks = 0
  let restoreBobAckTransport = false

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

    // The node has delivered the encrypted batch, but the acknowledgement is
    // unavailable. Bob must first persist decrypted messages, dedupe records,
    // unread state, and the advanced Ratchet session before a refresh.
    await openMe(bob)
    await bobContext.route('http://127.0.0.1:8787/api/mailbox/ack', async (route) => {
      if (restoreBobAckTransport) {
        await route.continue()
        return
      }
      blockedAcks += 1
      await route.abort('connectionrefused')
    })
    const persistedMessagesBefore = await persistedTableCount(bob, 'messages')
    await openOnlyContactConversation(alice)
    const batch = ['接收刷新第一条', '📥', '接收刷新第三条']
    for (const text of batch) {
      await alice.getByLabel('输入消息').fill(text)
      await alice.getByRole('button', { name: '发送' }).click()
    }
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(batch.length)
    // The route above intentionally rejects ACK. Mailbox processing may still
    // persist the decrypted messages before that expected ACK error surfaces.
    await takeMailbox(bob).catch(() => undefined)
    await expect.poll(() => blockedAcks, { timeout: 45_000 }).toBeGreaterThanOrEqual(1)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(bob, 'messages'), { timeout: 45_000 }).toBe(persistedMessagesBefore + batch.length)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(batch.length)

    // A new login reads the same real node deliveries. Its stored protocol ids
    // must suppress duplicate rendering, then allow the normal replacement ACK.
    restoreBobAckTransport = true
    await reloadAndLogin(bob, bobPassphrase)
    await takeMailbox(bob)
    await expect.poll(() => mailboxDeliveryTotal(bob, bobUserId), { timeout: 45_000 }).toBe(0)

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.locator('.bubble.in .text')).toHaveText(batch, { timeout: 45_000 })
    for (const text of batch) {
      await expect(bobMessages.getByText(text, { exact: true })).toHaveCount(1)
    }
    await expect(bob.locator('.conversation-badge')).toHaveCount(0)

    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(
      Array(batch.length).fill('已读'),
      { timeout: 45_000 },
    )
    await bob.getByLabel('输入消息').fill('接收刷新后继续收发')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('接收刷新后继续收发', { exact: true })).toHaveCount(1, { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})

test('删除本地会话后可由新消息恢复并继续使用既有 Ratchet 会话', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alicePassphrase = 'playwright-delete-conversation-alice-passphrase'
  const bobPassphrase = 'playwright-delete-conversation-bob-passphrase'
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
    await flushLocalPersistence(alice)
    await flushLocalPersistence(bob)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)
    await expect.poll(() => persistedTableCount(bob, 'ratchetSessions')).toBeGreaterThan(0)

    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await alice.getByLabel('输入消息').fill('删除前的本地消息')
    await alice.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('删除前的本地消息', { exact: true })).toBeVisible()
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.getByText('删除前的本地消息', { exact: true })).toBeVisible({ timeout: 45_000 })

    // Delete only the local transcript. The contact and persisted Ratchet
    // session must remain, so a later message can reopen this conversation.
    await alice.getByRole('button', { name: '更多' }).click()
    await alice.getByRole('menuitem', { name: '删除会话' }).click()
    const dialog = alice.getByRole('dialog')
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: '确定' }).click()
    await expect(alice).toHaveURL(/#\/chat$/)
    await expect(alice.locator('.contact')).toHaveCount(0)
    await expect.poll(() => persistedTableCount(alice, 'messages')).toBe(0)
    await expect.poll(() => persistedTableCount(alice, 'ratchetSessions')).toBeGreaterThan(0)

    await bob.getByLabel('输入消息').fill('删除后重新打开会话')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(alice.locator('.conversation-badge')).toHaveText('1', { timeout: 45_000 })
    await alice.locator('.contact').click()
    await expect(aliceMessages.getByText('删除后重新打开会话', { exact: true })).toHaveCount(1)
    await expect(alice.locator('.conversation-badge')).toHaveCount(0)

    await alice.getByLabel('输入消息').fill('会话恢复后继续发送')
    await alice.getByRole('button', { name: '发送' }).click()
    await expect(bobMessages.getByText('会话恢复后继续发送', { exact: true })).toHaveCount(1, { timeout: 45_000 })
    await expect(aliceMessages.locator('.bubble.out .message-status')).toHaveText(['已读'], { timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})
