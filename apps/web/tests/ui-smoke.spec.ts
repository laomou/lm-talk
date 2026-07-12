import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { accept_friend_request, initSync } from '../src/wasm/lm_wasm.js'

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

async function enableSync(page: Page) {
  await page.getByRole('button', { name: '我', exact: true }).click()
  await fieldAfterLabel(page, '同步服务').fill('http://sync.test')
  const button = page.getByRole('button', { name: '开启同步' })
  if (await button.isVisible()) await button.click()
  await expect(page.getByRole('button', { name: '关闭同步' })).toBeVisible()
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



async function copyMyBackup(page: Page): Promise<string> {
  await page.getByRole('button', { name: '我', exact: true }).click()
  await page.getByRole('button', { name: '导出身份' }).click()
  await expect(page.locator('.qr-modal')).toBeVisible()
  await page.locator('.qr-modal').getByRole('button', { name: '复制原文' }).click()
  const value = await page.evaluate(() => navigator.clipboard.readText())
  expect(value).toContain('lm-identity-backup-v1:')
  await page.locator('.qr-modal').getByRole('button', { name: '关闭' }).click()
  return value
}

async function openDetailsByText(page: Page, text: string) {
  const found = await page.evaluate((needle) => document.body.textContent?.includes(needle) ?? false, text)
  if (!found) { await page.getByRole('button', { name: '我', exact: true }).click(); await page.getByRole('button', { name: '诊断工具' }).click() }
  await page.evaluate((needle) => {
    for (const detail of [...document.querySelectorAll('details')] as HTMLDetailsElement[]) {
      if (detail.textContent?.includes(needle)) detail.open = true
    }
  }, text)
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

test('登录页创建身份后进入左联系人/群组、右聊天框布局', async ({ page }) => {
  await clearBrowserState(page)
  await page.reload()
  await expect(fieldAfterLabel(page, '提示词')).toBeVisible()

  await page.goto('/#/register')
  await fieldAfterLabel(page, '提示词').fill('我爱吃菠萝2026')
  await page.getByRole('button', { name: '注册', exact: true }).last().click({ force: true })
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()
  await expect(page.getByRole('button', { name: '下载身份' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click({ force: true })
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('我爱吃菠萝2026')
  await page.getByRole('button', { name: '登录', exact: true }).last().click()
  await expect(page.locator('.chat-shell')).toBeVisible()
  await page.getByRole('button', { name: '我', exact: true }).click()
  await fieldAfterLabel(page, '显示名', 'input').fill('Alice')
  await page.locator('.home-card').filter({ hasText: '显示名' }).getByRole('button', { name: '保存' }).click()
  await page.getByRole('button', { name: '聊天', exact: true }).click()

  await expect(page.locator('.chat-shell')).toBeVisible()
  await expect(page.locator('.sidebar')).toBeVisible()
  await expect(page.locator('.chat-main')).toBeVisible()
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
  await expect(page.getByRole('button', { name: '诊断工具' })).toBeVisible()
  await expect(page.locator('.app-shell')).not.toContainText('调试页面')
  await page.getByRole('button', { name: '聊天', exact: true }).click()
  await page.getByRole('button', { name: '我', exact: true }).click()
  await page.getByRole('button', { name: '诊断工具' }).click()
  await openDetailsByText(page, 'Public Peer / Mailbox 协议调试')
  await expect(page.locator('input[capture], video')).toHaveCount(0)
  await page.getByRole('button', { name: '我', exact: true }).click()
  await expect(page.locator('.home-card').filter({ hasText: '显示名' }).getByRole('button', { name: '保存' })).toBeVisible()
  await page.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(page.locator('.chat-empty-state').getByRole('heading', { name: '选择一个聊天' })).toBeVisible()
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute('href', '/manifest.webmanifest')
  const swAvailable = await page.evaluate(() => 'serviceWorker' in navigator)
  expect(swAvailable).toBe(true)
})

test('两端可用复制粘贴流程完成好友确认并发送可复制密文', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alice = await aliceContext.newPage()
  const bob = await bobContext.newPage()
  await clearBrowserState(alice)
  await clearBrowserState(bob)

  await createIdentity(alice, 'Alice', 'alice passphrase 2026')
  await createIdentity(bob, 'Bob', 'bob passphrase 2026')

  const aliceCard = await copyMyContactCard(alice)
  const bobCard = await copyMyContactCard(bob)
  const aliceBackup = await copyMyBackup(alice)
  const bobBackup = await copyMyBackup(bob)

  await alice.getByRole('button', { name: '通讯录', exact: true }).click()
  await fieldAfterLabel(alice, '对方名片').fill(bobCard)
  await alice.getByRole('button', { name: '添加好友' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob' })).toBeVisible()
  await bob.getByRole('button', { name: '我', exact: true }).click()
  await fieldAfterLabel(bob, '显示名', 'input').fill('Bob 新名')
  await bob.locator('.home-card').filter({ hasText: '显示名' }).getByRole('button', { name: '保存' }).click()
  const bobUpdatedCard = await copyMyContactCard(bob)
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await alice.getByRole('button', { name: '通讯录', exact: true }).click()
  await fieldAfterLabel(alice, '对方名片').fill(bobUpdatedCard)
  await alice.getByRole('button', { name: '添加好友' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob 新名' })).toBeVisible()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob 新名' })).toBeVisible()
  await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).click()
  await alice.getByRole('button', { name: '发送好友请求' }).click()
  await expect(alice.getByText('请先开启消息同步')).toBeVisible()
  await alice.getByRole('button', { name: '知道了' }).click({ force: true })
  await openDetailsByText(alice, '交换区：好友请求 / 收到的密文 Envelope')
  await alice.getByRole('button', { name: '生成好友请求' }).click()
  await alice.getByRole('button', { name: '复制好友请求' }).click()
  const friendRequest = await alice.evaluate(() => navigator.clipboard.readText())
  expect(friendRequest).toContain('lm-friend-request-v1:')
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: '等待通过' })).toBeVisible()

  initSync({ module: readFileSync(new URL('../src/wasm/lm_wasm_bg.wasm', import.meta.url)) })
  const friendResponse = accept_friend_request(bobBackup, 'bob passphrase 2026', friendRequest)
  expect(friendResponse).toContain('lm-friend-response-v1:')

  await openDetailsByText(alice, '交换区：好友请求 / 收到的密文 Envelope')
  await alice.getByPlaceholder('lm-friend-response-v1:...').fill(friendResponse)
  await alice.getByRole('button', { name: '应用好友响应' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob 新名' })).toBeVisible()
  await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).click()

  await openDetailsByText(alice, '双棘轮状态调试（高级）')
  await alice.getByRole('button', { name: '为当前联系人生成测试状态对' }).click()
  await expect.poll(async () => alice.evaluate(() => ([...document.querySelectorAll('textarea')] as HTMLTextAreaElement[]).map((x) => x.value).find((v) => v.startsWith('lm-ratchet-state-v1:')) ?? ''), { timeout: 10_000 }).toContain('lm-ratchet-state-v1:')
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).click()

  await alice.getByPlaceholder('输入消息').fill('你好 Bob，P2P 密文测试')
  await alice.getByRole('button', { name: '发送', exact: true }).click()
  await expect(alice.locator('.bubble.out')).toContainText('你好 Bob')
  const bobUserIdLine = await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).locator('small').innerText()
  const bobUserId = bobUserIdLine.split(' · ').pop() || bobUserIdLine
  await openDetailsByText(alice, '交换区：好友请求 / 收到的密文 Envelope')
  const envelope = await alice.locator('details').filter({ hasText: '交换区：好友请求 / 收到的密文 Envelope' }).locator('label:has-text("收到的 Envelope JSON") + textarea').inputValue()
  expect(envelope).toContain('x3dh-double-ratchet-v1')
  const sentMessageId = JSON.parse(envelope).message_id
  await openDetailsByText(alice, 'Public Peer / Mailbox 协议调试')
  await alice.locator('textarea[placeholder="mailbox messages"]').fill(JSON.stringify({
    messages: [{
      delivery_id: 'ack-delivery-1',
      message: {
        kind: 'Other',
        from_user_id: bobUserId,
        ciphertext: JSON.stringify({ type: 'lm-delivery-ack-v1', version: 1, message_id: sentMessageId, from_user_id: bobUserId, created_at: Date.now() }),
      },
    }],
  }))
  await alice.getByRole('button', { name: '处理下方 mailbox JSON' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).click()
  await expect(alice.locator('.bubble.out')).toContainText('已送达')

  await expect.poll(async () => {
    const messages = await idbStoreAll(alice, 'messages')
    return messages.some((m) => m.text?.__lm_enc_v1 === true && JSON.stringify(m).includes('__lm_enc_v1') && !JSON.stringify(m).includes('你好 Bob'))
  }, { timeout: 10_000 }).toBe(true)
  await expect.poll(async () => {
    const contacts = await idbStoreAll(alice, 'contacts')
    return contacts.some((c) => c.display_name?.__lm_enc_v1 === true && c.contact_card_text?.__lm_enc_v1 === true && !JSON.stringify(c).includes('Bob'))
  }, { timeout: 10_000 }).toBe(true)

  await alice.reload()
  await expect(fieldAfterLabel(alice, '提示词')).toBeVisible()
  await expect(alice.getByText('Alice', { exact: true })).toBeVisible()
  await fieldAfterLabel(alice, '提示词').fill('alice passphrase 2026')
  await alice.getByRole('button', { name: '登录', exact: true }).last().click()
  await expect(alice.locator('.chat-shell')).toBeVisible()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob' })).toBeVisible()
  await alice.locator('.contact').filter({ hasText: 'Bob' }).click()
  await expect(alice.locator('.bubble.out')).toContainText('你好 Bob')

  await openDetailsByText(alice, '文件传输 MVP')
  const fileInput = alice.locator('details').filter({ hasText: '文件传输 MVP' }).locator('input[type="file"]')
  await fileInput.setInputFiles({ name: 'hello.txt', mimeType: 'text/plain', buffer: Buffer.from('hello encrypted file') })
  await alice.getByRole('button', { name: '加密文件包' }).click()
  let filePackage = ''
  await expect.poll(async () => {
    filePackage = await alice.evaluate(() => ([...document.querySelectorAll('textarea')] as HTMLTextAreaElement[]).map((x) => x.value).find((v) => v.includes('lm-file-package-v1')) ?? '')
    return filePackage
  }, { timeout: 10_000 }).toContain('lm-file-package-v1')
  await expect(alice.getByText('文件包已生成')).toBeVisible()

  await bob.getByRole('button', { name: '通讯录', exact: true }).click()
  await fieldAfterLabel(bob, '对方名片').fill(aliceCard)
  await bob.getByRole('button', { name: '添加好友' }).click()
  await bob.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(bob.locator('.contact').filter({ hasText: 'Alice' })).toBeVisible()
  await bob.locator('.contact').filter({ hasText: 'Alice' }).click()
  await openDetailsByText(bob, '文件传输 MVP')
  await bob.locator('label:has-text("收到的文件包 JSON") + textarea').fill(filePackage)
  await bob.getByRole('button', { name: '解密文件包' }).click()
  await expect(bob.getByText('下载解密文件：hello.txt')).toBeVisible()
  await bob.getByRole('button', { name: '聊天', exact: true }).click()
  await bob.locator('.contact').filter({ hasText: 'Alice' }).click()
  await expect(bob.locator('.bubble.in')).toContainText('[文件] hello.txt')

  await alice.getByRole('button', { name: '通讯录', exact: true }).click()
  await alice.getByPlaceholder('例如：测试群').fill('测试群')
  await alice.locator('label.check-row').filter({ hasText: 'Bob' }).locator('input[type="checkbox"]').check()
  await alice.locator('.home-card').filter({ hasText: '群名' }).getByRole('button', { name: '创建群聊' }).click()
  await alice.getByRole('button', { name: '聊天', exact: true }).click()
  await expect(alice.locator('.contact').filter({ hasText: '测试群' })).toBeVisible()
  await alice.locator('.contact').filter({ hasText: '测试群' }).click()
  await expect(alice.getByRole('heading', { name: '测试群' })).toBeVisible()
  await alice.getByPlaceholder('输入消息').fill('群聊 smoke 测试')
  await alice.getByRole('button', { name: '发送', exact: true }).click()
  await expect(alice.locator('.bubble.out')).toContainText('群聊 smoke 测试')
  await expect.poll(async () => {
    await alice.evaluate(async () => { await (window as any).flushPersistForTests?.() })
    const outbox = await idbStoreAll(alice, 'outbox')
    return outbox.some((o) => o.kind === 'group-fanout' && o.envelope_json?.__lm_enc_v1 === true)
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
  await expect(page.getByRole('heading', { name: '导入身份', level: 1 })).toBeVisible()
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


test('消息同步可完成真实好友请求收发', async ({ browser }) => {
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
  await expect(alice.getByPlaceholder('输入消息')).toBeVisible()

  await aliceContext.close()
  await bobContext.close()
})
