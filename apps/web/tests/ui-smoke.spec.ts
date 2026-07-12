import { expect, test, type Locator, type Page } from '@playwright/test'
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






async function copyMyContactCard(page: Page): Promise<string> {
  await page.getByRole('button', { name: '卡片二维码' }).click()
  await expect(page.locator('.qr-modal')).toBeVisible()
  await page.locator('.qr-modal').getByRole('button', { name: '复制原文' }).click()
  const value = await page.evaluate(() => navigator.clipboard.readText())
  expect(value).toContain('lm-contact-card-v1:')
  await page.locator('.qr-modal').getByRole('button', { name: '关闭' }).click()
  return value
}



async function copyMyBackup(page: Page): Promise<string> {
  await page.getByRole('button', { name: '备份包二维码' }).click()
  await expect(page.locator('.qr-modal')).toBeVisible()
  await page.locator('.qr-modal').getByRole('button', { name: '复制原文' }).click()
  const value = await page.evaluate(() => navigator.clipboard.readText())
  expect(value).toContain('lm-identity-backup-v1:')
  await page.locator('.qr-modal').getByRole('button', { name: '关闭' }).click()
  return value
}

async function openDetailsByText(page: Page, text: string) {
  await page.evaluate((needle) => {
    const debugGate = [...document.querySelectorAll('details')] as HTMLDetailsElement[]
    for (const detail of debugGate) {
      if (detail.textContent?.includes('高级调试入口')) {
        detail.open = true
        const panel = detail.querySelector('.debug-panel') as HTMLElement | null
        if (panel) panel.style.display = ''
      }
    }
    for (const detail of [...document.querySelectorAll('details')] as HTMLDetailsElement[]) {
      if (detail.textContent?.includes(needle)) detail.open = true
    }
  }, text)
}

async function createIdentity(page: Page, name: string, passphrase: string) {
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'LM Talk' })).toBeVisible()
  await page.getByRole('button', { name: '注册', exact: true }).click()
  await fieldAfterLabel(page, '提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册', exact: true }).last().click()
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()
  await expect(page.getByRole('button', { name: '下载身份' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click()
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录', exact: true }).last().click()
  await expect(page.locator('.chat-shell')).toBeVisible()
  await fieldAfterLabel(page, '我的显示名', 'input').fill(name)
  await page.getByRole('button', { name: '更新我的卡片' }).click()
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

  await page.getByRole('button', { name: '注册', exact: true }).click()
  await fieldAfterLabel(page, '提示词').fill('我爱吃菠萝2026')
  await page.getByRole('button', { name: '注册', exact: true }).last().click()
  await expect(page.getByRole('heading', { name: '注册成功' })).toBeVisible()
  await expect(page.getByRole('button', { name: '下载身份' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click()
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('我爱吃菠萝2026')
  await page.getByRole('button', { name: '登录', exact: true }).last().click()
  await fieldAfterLabel(page, '我的显示名', 'input').fill('Alice')
  await page.getByRole('button', { name: '更新我的卡片' }).click()

  await expect(page.locator('.chat-shell')).toBeVisible()
  await expect(page.locator('.sidebar')).toBeVisible()
  await expect(page.locator('.chat-main')).toBeVisible()
  await expect(page.locator('.sidebar').getByRole('heading', { name: '联系人', exact: true })).toBeVisible()
  await expect(page.locator('.sidebar').getByRole('heading', { name: '群组', exact: true })).toBeVisible()
  await expect(page.getByRole('heading', { name: '请选择联系人或群组' })).toBeVisible()
  await page.getByText('安全会话建立（推荐）').click()
  await expect(page.getByText('无服务器复制粘贴流程')).toBeVisible()
  await page.getByText('高级调试入口（默认隐藏）').click()
  await page.getByText('Public Peer / Mailbox 协议调试').click()
  await expect(page.getByText('不使用摄像头')).toBeVisible()
  await expect(page.locator('input[capture], video')).toHaveCount(0)
  await expect(page.getByRole('button', { name: '更新我的卡片' })).toBeVisible()
  await expect(page.getByLabel('自动从 Peer 节点同步 snapshot')).toBeVisible()
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

  await fieldAfterLabel(alice, '添加联系人 Contact Card').fill(bobCard)
  await alice.getByRole('button', { name: '添加联系人' }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob' })).toBeVisible()
  await fieldAfterLabel(bob, '我的显示名', 'input').fill('Bob 新名')
  await bob.getByRole('button', { name: '更新我的卡片' }).click()
  const bobUpdatedCard = await copyMyContactCard(bob)
  await fieldAfterLabel(alice, '添加联系人 Contact Card').fill(bobUpdatedCard)
  await alice.getByRole('button', { name: '添加联系人' }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Bob 新名' })).toBeVisible()
  await expect(alice.locator('.contact').filter({ hasText: 'LocalOnly' })).toBeVisible()
  await alice.getByRole('button', { name: '生成好友请求' }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'RequestSent' })).toBeVisible()
  await openDetailsByText(alice, '交换区：好友请求 / 收到的密文 Envelope')
  let friendRequest = ''
  await expect.poll(async () => {
    friendRequest = await alice.evaluate(() => ([...document.querySelectorAll('textarea')] as HTMLTextAreaElement[]).map((x) => x.value).find((v) => v.startsWith('lm-friend-request-v1:')) ?? '')
    return friendRequest
  }, { timeout: 10_000 }).toContain('lm-friend-request-v1:')

  initSync({ module: readFileSync(new URL('../src/wasm/lm_wasm_bg.wasm', import.meta.url)) })
  const friendResponse = accept_friend_request(bobBackup, 'bob passphrase 2026', friendRequest)
  expect(friendResponse).toContain('lm-friend-response-v1:')

  await alice.locator('label:has-text("收到的好友响应") + textarea').fill(friendResponse)
  await alice.getByRole('button', { name: '应用好友响应' }).click()
  await expect(alice.locator('.contact').filter({ hasText: 'Friend' })).toBeVisible()

  await openDetailsByText(alice, '双棘轮状态调试（高级）')
  await alice.getByRole('button', { name: '为当前联系人生成测试状态对' }).click()
  await expect.poll(async () => alice.evaluate(() => ([...document.querySelectorAll('textarea')] as HTMLTextAreaElement[]).map((x) => x.value).find((v) => v.startsWith('lm-ratchet-state-v1:')) ?? ''), { timeout: 10_000 }).toContain('lm-ratchet-state-v1:')

  await alice.locator('footer.composer textarea').fill('你好 Bob，P2P 密文测试')
  await alice.getByRole('button', { name: '发送/生成密文' }).click()
  await expect(alice.locator('.bubble.out')).toContainText('你好 Bob')
  const envelope = await alice.evaluate(() => ([...document.querySelectorAll('.bubble textarea, textarea')] as HTMLTextAreaElement[]).map((x) => x.value).find((v) => v.includes('x25519-static-hkdf-xchacha20poly1305-v1') || v.includes('x3dh-double-ratchet-v1')) ?? '')
  expect(envelope).toContain('x3dh-double-ratchet-v1')
  const sentMessageId = JSON.parse(envelope).message_id
  const bobUserId = await alice.locator('.contact').filter({ hasText: 'Bob 新名' }).locator('small').innerText()
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

  await fieldAfterLabel(bob, '添加联系人 Contact Card').fill(aliceCard)
  await bob.getByRole('button', { name: '添加联系人' }).click()
  await expect(bob.locator('.contact').filter({ hasText: 'Alice' })).toBeVisible()
  await openDetailsByText(bob, '文件传输 MVP')
  await bob.locator('label:has-text("收到的文件包 JSON") + textarea').fill(filePackage)
  await bob.getByRole('button', { name: '解密文件包' }).click()
  await expect(bob.getByText('下载解密文件：hello.txt')).toBeVisible()
  await expect(bob.locator('.bubble.in')).toContainText('[文件] hello.txt')

  await alice.getByPlaceholder('例如：测试群').fill('测试群')
  await alice.locator('label.check-row').filter({ hasText: 'Bob' }).locator('input[type="checkbox"]').check()
  await alice.locator('.add-box').filter({ hasText: '群名' }).getByRole('button', { name: '创建群' }).click()
  await expect(alice.locator('.contact').filter({ hasText: '测试群' })).toBeVisible()
  await alice.locator('.contact').filter({ hasText: '测试群' }).click()
  await expect(alice.getByRole('heading', { name: '测试群' })).toBeVisible()
  await alice.locator('footer.composer textarea').fill('群聊 smoke 测试')
  await alice.getByRole('button', { name: '发送/生成密文' }).click()
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
  await page.getByRole('button', { name: '注册', exact: true }).click()
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
  await page.getByRole('button', { name: '导入', exact: true }).click()
  await expect(page.getByRole('heading', { name: '导入身份', level: 1 })).toBeVisible()
  await expect(page.getByText('提示词不正确，请重新输入。')).toBeVisible()
  await page.getByRole('button', { name: '知道了' }).click()

  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '导入', exact: true }).click()
  await expect(page.getByRole('heading', { name: '登录 LM Talk' })).toBeVisible()
  await expect(page.getByText('Me', { exact: true })).toBeVisible()
  await fieldAfterLabel(page, '提示词').fill('import passphrase 2026')
  await page.getByRole('button', { name: '登录', exact: true }).click()
  await expect(page.locator('.chat-shell')).toBeVisible()
})
