import { expect, test, type Page } from '@playwright/test'

async function waitForWasm(page: Page) {
  await expect(page.getByText('正在准备')).toBeHidden({ timeout: 30_000 })
}

async function registerAndLogin(page: Page, passphrase: string) {
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册' }).click()
  await expect(page.getByRole('heading', { name: '身份已创建' })).toBeVisible()
  await page.getByRole('button', { name: '去登录' }).click()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/#\/chat$/)
}

function contactRows(page: Page) {
  return page.locator('.directory-row.contact-row')
}

function makeContacts() {
  const contacts: Array<{ user_id: string; display_name: string }> = []
  for (let i = 0; i < 60; i += 1) contacts.push({ user_id: `lm-test-alpha-${i}`, display_name: `Alpha ${String(i).padStart(3, '0')}` })
  for (let i = 0; i < 20; i += 1) contacts.push({ user_id: `lm-test-beta-${i}`, display_name: `Beta ${String(i).padStart(3, '0')}` })
  for (let i = 0; i < 10; i += 1) contacts.push({ user_id: `lm-test-zeta-${i}`, display_name: `Zeta ${String(i).padStart(3, '0')}` })
  contacts.push({ user_id: 'lm-test-number', display_name: '123 User' })
  contacts.push({ user_id: 'lm-test-zhangsan', display_name: '张三' })
  contacts.push({ user_id: 'lm-test-emoji', display_name: '😀 Emoji' })
  return contacts
}

test('大量联系人支持分组索引、搜索限制和返回恢复', async ({ page }) => {
  await registerAndLogin(page, 'playwright-contacts-scale-passphrase')
  const contacts = makeContacts()
  await page.evaluate((items) => {
    const win = window as typeof window & {
      seedFriendContactsForTests?: (contacts: Array<{ user_id: string; display_name: string }>) => void
      flushPersistForTests?: () => Promise<void>
    }
    win.seedFriendContactsForTests?.(items)
  }, contacts)
  await page.evaluate(async () => {
    await (window as typeof window & { flushPersistForTests?: () => Promise<void> }).flushPersistForTests?.()
  })

  await page.getByRole('button', { name: '打开通讯录' }).click()
  await expect(page).toHaveURL(/#\/contacts$/)
  await expect(contactRows(page)).toHaveCount(contacts.length)
  await expect(page.getByRole('heading', { name: 'A' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'B' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Z' })).toBeVisible()
  await expect(page.getByRole('heading', { name: '#' })).toBeVisible()

  await page.getByRole('button', { name: '跳到 Z 组' }).click()
  await expect(page.getByRole('heading', { name: 'Z' })).toBeVisible()

  await page.getByRole('button', { name: '搜索联系人' }).click()
  await expect(page).toHaveURL(/#\/contacts\/search$/)
  await page.getByLabel('搜索联系人').fill('Alpha')
  await expect(page.getByText('仅显示前 50 个联系人')).toBeVisible({ timeout: 45_000 })
  await expect(contactRows(page)).toHaveCount(50)

  await page.getByLabel('搜索联系人').fill('lm-test-zeta-3')
  await expect(page.getByText('共找到 1 个联系人')).toBeVisible({ timeout: 45_000 })
  await expect(contactRows(page)).toHaveCount(1)
  await contactRows(page).first().click()
  await expect(page.getByRole('heading', { name: 'Zeta 003' })).toBeVisible()
  await page.getByRole('button', { name: '返回通讯录' }).click()
  await expect(page).toHaveURL(/#\/contacts$/)
  await expect(contactRows(page)).toHaveCount(contacts.length)
})
