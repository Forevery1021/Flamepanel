import { test, expect } from '@playwright/test'
import { E2E_ADMIN, collectErrors, login } from './helpers/flow'

/**
 * S-14：新手引导（UX-2）在真实浏览器里的完整生命周期。
 *
 * 首登弹出 → Esc 跳过 → 刷新不再弹（完成标记按用户名落在 localStorage）→
 * 用户菜单「重新播放引导」可重放。jsdom 单测覆盖了 store 的状态机，但「Esc 落在
 * 聚焦的按钮上能否冒泡到对话框」「挂载时机与仪表盘渲染的先后」只有真浏览器能验证。
 *
 * 登录预算：本文件 1 次（见 helpers/flow.ts 顶部说明；错误采集从登录后开始）。
 */
test.describe('S-14：新手引导', () => {
  test('首登弹出 → Esc 跳过 → 刷新不再弹 → 用户菜单重放', async ({ page }) => {
    // 全新上下文 = 全新 localStorage：对该用户而言就是「首次进入」
    await login(page, E2E_ADMIN, { keepTour: true })
    const errors = collectErrors(page)

    const tour = page.getByRole('dialog', { name: '新手引导' })
    await expect(tour).toBeVisible()
    await expect(tour).toContainText('欢迎使用 FlamePanel')

    // Esc 跳过：焦点在引导卡片的按钮上，keydown 冒泡到对话框
    await page.keyboard.press('Escape')
    await expect(tour).toBeHidden()
    const mark = await page.evaluate(
      (u) => localStorage.getItem(`flamepanel.onboarding.${u}`),
      E2E_ADMIN.username,
    )
    expect(mark, '跳过后应按用户名写入完成标记').toContain('"outcome":"skipped"')

    // 刷新：仪表盘渲染完成时引导已挂载——此刻仍不可见即「不再弹」
    await page.reload()
    await expect(page.locator('.stat-card')).toHaveCount(4)
    await expect(tour).toHaveCount(0)

    // 用户菜单重放：从第一步开始，可前进，也可再次跳过
    await page.getByRole('button', { name: '用户菜单' }).click()
    await page.getByRole('menuitem', { name: '重新播放引导' }).click()
    await expect(tour).toBeVisible()
    await expect(tour).toContainText('欢迎使用 FlamePanel')
    await tour.getByRole('button', { name: '开始导览' }).click()
    await expect(tour).toContainText('侧边导航')
    await tour.getByRole('button', { name: '跳过' }).click()
    await expect(tour).toBeHidden()

    errors.assertClean('新手引导弹出 / 跳过 / 重放')
  })
})
