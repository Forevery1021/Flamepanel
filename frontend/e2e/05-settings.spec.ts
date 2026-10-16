import { test, expect, type Request } from '@playwright/test'
import { collectErrors, gotoMenu, login, navigateViaPalette } from './helpers/flow'

/**
 * S-14：设置页三条真实浏览器契约。
 *
 * 1. **分区深链**：`/settings#security` 直达「安全」分区（hash 即分区，可分享可刷新）；
 * 2. **脏状态离开确认**：改了未保存 → 应用内跳转被拦下确认，「取消」留在原页且改动仍在；
 * 3. **批量保存一次请求**：同一分区改多项，保存只发一次 `PATCH /api/settings/batch`，
 *    载荷只含变更键（后端原子写入，避免半成功）。保存后不再脏，跳转不再拦。
 *
 * 离开守卫只对 vue-router 跳转生效，故跳转走命令面板（`navigateViaPalette`），不用 `page.goto`。
 * 登录预算：本文件 1 次（见 helpers/flow.ts 顶部说明；错误采集从登录后开始）。
 */
test.describe('S-14：设置', () => {
  test('#security 深链 / 未保存离开确认 / 批量保存一次请求', async ({ page }) => {
    await login(page)
    const errors = collectErrors(page)

    // ① 深链直达安全分区
    await page.goto('/settings#security')
    await expect(page.locator('.lc-title__text')).toHaveText('面板设置')
    await expect(page.getByRole('tab', { name: /安全/ })).toHaveAttribute('aria-selected', 'true')
    await expect(page.getByRole('heading', { name: '修改密码' })).toBeVisible()

    // 切到面板配置：hash 同步（同页切换不触发离开确认）
    await page.getByRole('tab', { name: /面板配置/ }).click()
    await expect(page).toHaveURL(/\/settings#general$/)

    // ② 改面板名未保存 → 应用内跳转被拦
    const panelName = page.getByLabel('面板名称', { exact: true })
    await panelName.fill('FlamePanel E2E')
    await expect(page.getByText('更改尚未保存')).toBeVisible()

    await navigateViaPalette(page, '备忘录')
    const confirm = page.getByRole('alertdialog').filter({ hasText: '有未保存的更改' })
    await expect(confirm).toBeVisible()
    await confirm.getByRole('button', { name: '取消' }).click()
    await expect(confirm).toBeHidden()
    await expect(page).toHaveURL(/\/settings#general$/)
    await expect(panelName).toHaveValue('FlamePanel E2E')

    // ③ 再改一项（日志保留天数），保存只发一次批量请求
    // SettingRow 的标题即控件的 <label for>
    const retention = page.getByLabel('日志保留天数', { exact: true })
    await retention.click()
    await retention.press('Control+a')
    await retention.pressSequentially('45')
    await retention.press('Tab')

    const writes: Request[] = []
    page.on('request', (r) => {
      if (/\/api\/settings/.test(r.url()) && r.method() !== 'GET') writes.push(r)
    })
    const saved = page.waitForResponse(
      (r) => r.url().endsWith('/api/settings/batch') && r.request().method() === 'PATCH',
    )
    await page.getByRole('button', { name: '保存设置' }).click()
    expect((await saved).ok()).toBe(true)
    await expect(page.getByText('更改尚未保存')).toBeHidden()

    expect(writes, '批量保存应只发一次写请求').toHaveLength(1)
    const body = writes[0]!.postDataJSON() as { settings: [string, string][] }
    expect(Object.fromEntries(body.settings)).toEqual({
      panel_name: 'FlamePanel E2E',
      log_retention_days: '45',
    })

    // 已保存：同样的跳转不再拦
    await navigateViaPalette(page, '备忘录')
    await expect(page).toHaveURL(/\/memos$/)
    await expect(page.locator('.lc-title__text')).toHaveText('备忘录')

    // 刷新回设置：服务端真值已落库
    await gotoMenu(page, '/settings#general', '面板设置')
    await expect(page.getByLabel('面板名称', { exact: true })).toHaveValue('FlamePanel E2E')

    errors.assertClean('设置深链 / 离开确认 / 批量保存')
  })
})
