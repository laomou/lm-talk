import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test'

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
  return page.locator(`label:has-text("${labelText}") + ${tag}`).first()
}

type MockMailboxMessage = { delivery_id: string; message: any }

function decodePrefixedJson(text: string): any {
  const payload = text.includes(':') ? text.slice(text.indexOf(':') + 1) : text
  const normalized = payload.replace(/-/g, '+').replace(/_/g, '/')
  const padded = normalized + '='.repeat((4 - normalized.length % 4) % 4)
  return JSON.parse(Buffer.from(padded, 'base64').toString('utf8'))
}

async function installMockSyncNode(context: BrowserContext, mailboxes: Map<string, MockMailboxMessage[]>) {
  await context.route('**/*', async (route) => {
    const req = route.request()
    const url = new URL(req.url())
    if (url.hostname !== 'sync.test') return route.continue()
    if (url.pathname === '/health') return route.fulfill({ json: { ok: true } })
    if (url.pathname === '/prekey/publish') return route.fulfill({ json: { ok: true } })
    if (url.pathname === '/mailbox/ack') return route.fulfill({ json: { ok: true } })
    if (url.pathname === '/mailbox/push') {
      const body = req.postDataJSON() as { message_text: string }
      const message = decodePrefixedJson(body.message_text)
      const deliveryId = `mock-${Date.now()}-${Math.random().toString(36).slice(2)}`
      const key = String(message.to_user_id)
      mailboxes.set(key, [...(mailboxes.get(key) ?? []), { delivery_id: deliveryId, message }])
      return route.fulfill({ json: { delivery_id: deliveryId } })
    }
    if (url.pathname === '/mailbox/take') {
      const userId = url.searchParams.get('user_id') || ''
      const messages = mailboxes.get(userId) ?? []
      mailboxes.set(userId, [])
      return route.fulfill({ json: { messages } })
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
  await page.getByRole('button', { name: '登录', exact: true }).last().click()
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.getByRole('button', { name: '我', exact: true }).click()
  await fieldAfterLabel(page, '显示名', 'input').fill(name)
  await page.locator('.home-card').filter({ hasText: '显示名' }).getByRole('button', { name: '保存' }).click()
  await page.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(page.locator('.me h2')).toHaveText(name)
}

async function copyMyContactCard(page: Page): Promise<string> {
  await page.getByRole('button', { name: '我', exact: true }).click()
  await page.getByRole('button', { name: '我的名片' }).click()
  await expect(page.locator('.qr-modal')).toBeVisible()
  await page.locator('.qr-modal').getByRole('button', { name: '复制原文' }).click()
  const value = await page.evaluate(() => navigator.clipboard.readText())
  expect(value).toContain('lm-contact-card-v1:')
  await page.locator('.qr-modal').getByRole('button', { name: '关闭' }).click()
  return value
}

async function enableSync(page: Page) {
  await page.getByRole('button', { name: '我', exact: true }).click()
  await fieldAfterLabel(page, '同步服务').fill('http://sync.test')
  const button = page.getByRole('button', { name: '开启同步' })
  if (await button.isVisible()) await button.click()
  await expect(page.getByRole('button', { name: '关闭同步' })).toBeVisible()
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

  await page.getByRole('button', { name: '通讯录', exact: true }).click()
  await expect(page.getByRole('button', { name: '刷新' })).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('粘贴好友请求')
  await expect(page.locator('.app-shell')).not.toContainText('粘贴群邀请')
  await expect(page.locator('.app-shell')).not.toContainText('生成邀请')

  await page.getByRole('button', { name: '我', exact: true }).click()
  await expect(page.getByRole('button', { name: '立即同步' })).toBeVisible()
  await expect(page.getByText('同步状态')).toBeVisible()
  await expect(page.getByText('最近记录')).toBeVisible()
  await expect(page.getByRole('button', { name: '诊断工具' })).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('调试页面')
  await expect(page.locator('.app-shell')).not.toContainText('开发协议工具')

  await page.getByRole('button', { name: '诊断工具' }).click()
  await expect(page.getByRole('heading', { name: '诊断工具' })).toBeVisible()
  await expect(page.getByText('一键诊断')).toBeVisible()
  await page.getByRole('button', { name: '生成诊断报告' }).click()
  await expect.poll(async () => page.locator('.diagnostic-actions textarea').inputValue()).toContain('service_worker')

  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute('href', '/manifest.webmanifest')
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

  await alice.getByRole('button', { name: '通讯录', exact: true }).click()
  await fieldAfterLabel(alice, '对方名片').fill(bobCard)
  await alice.getByRole('button', { name: '添加好友' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.contact').filter({ hasText: '等待通过' })).toBeVisible()

  await bob.getByRole('button', { name: '通讯录', exact: true }).click()
  await bob.getByRole('button', { name: '刷新' }).click()
  await expect(bob.getByRole('button', { name: '接受' })).toBeVisible()
  await bob.getByRole('button', { name: '接受' }).click()
  await expect(bob.locator('.contact-detail-card')).toContainText('Alice')

  await alice.getByRole('button', { name: '通讯录', exact: true }).click()
  await alice.getByRole('button', { name: '刷新' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.clean-chat-header')).toContainText('好友')
  await alice.getByPlaceholder('输入消息').fill('你好 Bob')
  await alice.getByRole('button', { name: '发送', exact: true }).click()
  await expect(alice.locator('.bubble.out')).toContainText('你好 Bob')

  await bob.getByRole('button', { name: '通讯录', exact: true }).click()
  await bob.getByRole('button', { name: '刷新' }).click()
  await bob.getByRole('button', { name: '聊天', exact: true }).click()
  await bob.locator('.contact').filter({ hasText: 'Alice' }).click()
  await expect(bob.locator('.bubble.in')).toContainText('你好 Bob')

  await expect.poll(async () => {
    const messages = await idbStoreAll(alice, 'messages')
    return messages.some((m) => m.text?.__lm_enc_v1 === true && JSON.stringify(m).includes('__lm_enc_v1') && !JSON.stringify(m).includes('你好 Bob'))
  }, { timeout: 10_000 }).toBe(true)

  await aliceContext.close()
  await bobContext.close()
})

test('注册后可在独立导入页导入身份，再回登录页登录', async ({ page }) => {
  await clearBrowserState(page)
  await page.goto('/#/register')
  await expect(page.getByRole('heading', { name: '注册 LM Talk' })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '注册', exact: true }).click({ force: true })
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()

  const identityText = await page.evaluate(() => {
    const records = JSON.parse(localStorage.getItem('lm-talk-local-identities-v1') || '[]') as Array<{ backup_text: string }>
    return records[0]?.backup_text || ''
  })
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
  await page.getByRole('button', { name: '登录', exact: true }).click()
  await expect(page.locator('.chat-shell')).toBeVisible()
})
