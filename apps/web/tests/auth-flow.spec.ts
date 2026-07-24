import { expect, test, type Page } from '@playwright/test'

const LOCAL_IDENTITIES_KEY = 'lm-talk-local-identities-v1'

async function clearBrowserState(page: Page) {
  await page.goto('/#/login')
  await page.evaluate(async () => {
    localStorage.clear()
    sessionStorage.clear()
    const databases = await indexedDB.databases?.()
    await Promise.all((databases ?? []).map((database) => database.name
      ? new Promise<void>((resolve) => {
          const request = indexedDB.deleteDatabase(database.name!)
          request.onsuccess = () => resolve()
          request.onerror = () => resolve()
          request.onblocked = () => resolve()
        })
      : Promise.resolve()))
  })
}

async function waitForWasm(page: Page) {
  await expect(page.getByText('正在准备')).toBeHidden({ timeout: 30_000 })
}

test.beforeEach(async ({ page }) => {
  await clearBrowserState(page)
})

test('注册后可下载身份并回到登录页', async ({ page }) => {
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill('playwright-register-passphrase')

  await page.getByRole('button', { name: '注册' }).click()
  await expect(page.getByRole('heading', { name: '身份已创建' })).toBeVisible()
  await expect(page.getByText('身份文件和提示词缺一不可')).toBeVisible()

  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: '下载身份' }).click()
  const identityFile = await download
  expect(identityFile.suggestedFilename()).toContain('.lm-identity-backup.txt')

  await page.getByRole('button', { name: '去登录' }).click()
  await expect(page.getByRole('heading', { name: '登录' })).toBeVisible()
  await expect(page.getByText('选择身份')).toBeVisible()
})

test('注册后可验证导入跳转到导入页', async ({ page }) => {
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill('playwright-verify-passphrase')

  await page.getByRole('button', { name: '注册' }).click()
  await expect(page.getByRole('heading', { name: '身份已创建' })).toBeVisible()

  await page.getByRole('button', { name: '验证导入' }).click()
  await expect(page.getByRole('heading', { name: '导入身份' })).toBeVisible()
  await expect(page.getByLabel('导入身份文本')).not.toBeEmpty()
})

test('登录页可取消或确认删除本地身份', async ({ page }) => {
  await page.evaluate((storageKey) => {
    localStorage.setItem(storageKey, JSON.stringify([{
      id: 'delete-test-user',
      user_id: 'delete-test-user',
      display_name: '删除测试',
      backup_text: 'lm-identity-backup-v1:test',
      updated_at: Date.now(),
    }]))
  }, LOCAL_IDENTITIES_KEY)
  await page.reload()
  await waitForWasm(page)

  await page.getByRole('button', { name: '删除本地身份' }).click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toBeVisible()
  await expect(dialog).toHaveText(/删除本地身份/)

  await dialog.getByRole('button', { name: '取消' }).click()
  await expect(dialog).toBeHidden()
  await expect(page.getByText('删除测试')).toBeVisible()

  await page.getByRole('button', { name: '删除本地身份' }).click()
  await dialog.getByRole('button', { name: '确定' }).click()
  await expect(page.getByText('还没有本机身份')).toBeVisible()
  await expect.poll(() => page.evaluate((storageKey) => localStorage.getItem(storageKey), LOCAL_IDENTITIES_KEY)).toBe('[]')
})

test('已登录用户刷新通讯录后，重新登录会回到通讯录', async ({ page }) => {
  const passphrase = 'playwright-refresh-passphrase'
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册' }).click()
  await page.getByRole('button', { name: '去登录' }).click()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/#\/chat$/)

  await page.getByRole('button', { name: '打开通讯录' }).click()
  await expect(page).toHaveURL(/#\/contacts$/)
  await expect(page.getByRole('heading', { name: '通讯录' })).toBeVisible()

  await page.reload()
  await waitForWasm(page)
  await expect(page).toHaveURL(/#\/contacts$/)
  await expect(page.getByRole('heading', { name: '登录' })).toBeVisible()

  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/#\/contacts$/)
  await expect(page.getByRole('heading', { name: '通讯录' })).toBeVisible()
})

test('导入身份页需要身份文本并可返回登录页', async ({ page }) => {
  await page.goto('/#/import')
  await waitForWasm(page)
  await expect(page.getByRole('heading', { name: '导入身份' })).toBeVisible()
  await expect(page.getByText('导入需要身份文本和对应提示词')).toBeVisible()
  await expect(page.getByRole('button', { name: '导入' })).toBeDisabled()

  await page.getByLabel('导入身份提示词').fill('wrong-passphrase')
  await page.getByLabel('导入身份文本').fill('not-a-valid-identity-package')
  await expect(page.getByRole('button', { name: '导入' })).toBeEnabled()

  await page.getByRole('button', { name: '返回登录' }).click()
  await expect(page.getByRole('heading', { name: '登录' })).toBeVisible()
})

test('移动端可从二维码图片识别名片并在添加前确认', async ({ page }) => {
  const passphrase = 'playwright-qr-scan-passphrase'
  await page.goto('/#/register')
  await waitForWasm(page)
  await page.getByLabel('注册提示词').fill(passphrase)
  await page.getByRole('button', { name: '注册' }).click()
  await page.getByRole('button', { name: '去登录' }).click()
  await page.getByLabel('登录提示词').fill(passphrase)
  await page.getByRole('button', { name: '登录' }).click()

  await page.locator('.rail-avatar[aria-label="打开我的设置"]').click()
  await page.getByText('个人资料', { exact: true }).click()
  await page.getByRole('button', { name: '我的名片' }).click()
  const qrDialog = page.getByRole('dialog')
  const qrDataUrl = await qrDialog.locator('img[alt="二维码"]').getAttribute('src')
  expect(qrDataUrl).toMatch(/^data:image\/png;base64,/)
  await qrDialog.getByRole('button', { name: '关闭', exact: true }).click()
  await page.getByRole('button', { name: '返回我' }).click()

  await page.setViewportSize({ width: 390, height: 844 })
  await page.getByRole('button', { name: '打开通讯录' }).click()
  await page.getByRole('button', { name: '添加好友' }).click()
  await page.getByRole('button', { name: '扫码添加' }).click()
  const scanner = page.getByRole('dialog', { name: '扫码添加' })
  await expect(scanner).toBeVisible()
  await expect(scanner.getByRole('button', { name: '从相册选择' })).toBeVisible()

  const base64 = qrDataUrl!.split(',')[1]
  await scanner.locator('input[type="file"]').setInputFiles({
    name: 'contact-card.png',
    mimeType: 'image/png',
    buffer: Buffer.from(base64, 'base64'),
  })
  await expect(scanner).toBeHidden()
  await expect(page.getByText('扫码结果', { exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: '添加好友' }).last()).toBeVisible()
  await page.getByRole('button', { name: '取消' }).last().click()
  await expect(page.getByText('扫码结果', { exact: true })).toBeHidden()
})
