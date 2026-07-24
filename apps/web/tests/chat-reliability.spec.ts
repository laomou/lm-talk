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

test('双用户可加好友、双向收发文字与 Emoji，并同步送达和已读回执', async ({ browser }) => {
  const aliceContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const bobContext = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] })
  const alice = await registerAndLogin(aliceContext, 'Alice', 'playwright-alice-passphrase')
  const bob = await registerAndLogin(bobContext, 'Bob', 'playwright-bob-passphrase')

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
    await openOnlyContactConversation(alice)
    const aliceMessages = alice.getByRole('log', { name: '消息列表' })
    await alice.getByLabel('输入消息').fill('你好 Bob，来自 Alice')
    await alice.getByRole('button', { name: '发送' }).click()
    await expect(aliceMessages.getByText('你好 Bob，来自 Alice')).toBeVisible()

    await openOnlyContactConversation(bob)
    const bobMessages = bob.getByRole('log', { name: '消息列表' })
    await expect(bobMessages.getByText('你好 Bob，来自 Alice')).toBeVisible({ timeout: 45_000 })

    await openOnlyContactConversation(alice)
    // A read receipt supersedes the visible "delivered" label. Bob receiving the
    // message plus Alice receiving the final read status covers the full flow.
    await expect(alice.getByRole('log', { name: '消息列表' }).getByText('已读', { exact: true })).toBeVisible({ timeout: 45_000 })

    await bob.getByLabel('输入消息').fill('🎉')
    await bob.getByRole('button', { name: '发送' }).click()
    await expect(bobMessages.getByText('🎉', { exact: true })).toBeVisible()

    await openOnlyContactConversation(alice)
    await expect(alice.getByRole('log', { name: '消息列表' }).getByText('🎉', { exact: true })).toBeVisible({ timeout: 45_000 })
  } finally {
    await aliceContext.close()
    await bobContext.close()
  }
})
