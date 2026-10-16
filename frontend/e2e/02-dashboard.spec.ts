import { test, expect } from '@playwright/test'
import { collectErrors, login } from './helpers/flow'

/**
 * 黄金路径②：登录 → Dashboard 指标渲染（Doc/23 §3.2 T1-2）。
 *
 * 断言落在「用户能看见的真实数值」上，而非「接口返回了 200」：
 * Dashboard 的四张指标卡由 **WebSocket 推送**（/ws/metrics）驱动，REST 只提供进程表。
 * 因此本用例同时是 WS 链路（首帧鉴权 → init/tick）在真实浏览器里的端到端验证——
 * jsdom 测不到 WebSocket，InMemory 也测不到真实采集循环。
 */
test.describe('黄金路径②：登录与仪表盘', () => {
  test('登录后进入仪表盘，四张指标卡与 WS 实时数据到位', async ({ page }) => {
    const errors = collectErrors(page)
    await login(page)
    await expect(page).toHaveURL(/\/dashboard$/)

    // 四张指标卡：CPU / 内存 / 磁盘 / 负载（名称为 zh-CN 译文）
    const cards = page.locator('.stat-card')
    await expect(cards).toHaveCount(4)
    await expect(cards.nth(0)).toContainText('CPU')
    await expect(cards.nth(1)).toContainText('内存')
    await expect(cards.nth(2)).toContainText('磁盘')
    await expect(cards.nth(3)).toContainText('系统负载')

    // 每张卡都有真实的百分比/数值（排除空壳渲染：数值须为 `12.3` / `12.3%` 形态）
    for (let i = 0; i < 4; i++) {
      const value = cards.nth(i).locator('.stat-value')
      await expect
        .poll(async () => /^\d+(?:\.\d+)?%?$/.test(((await value.textContent()) ?? '').trim()), {
          message: `第 ${i + 1} 张指标卡应渲染真实数值`,
          timeout: 20_000,
        })
        .toBe(true)
    }

    // WS 已连接 + 数据点开始累积（首屏 REST 不产生数据点，全由 /ws/metrics 驱动）
    await expect(page.locator('.ws-badge')).toContainText('已连接', { timeout: 20_000 })
    await expect
      .poll(
        async () => {
          const text = await page.locator('.info-row .mono').first().textContent()
          return Number(text?.trim() ?? 0)
        },
        { message: '等待 /ws/metrics 推送数据点', timeout: 20_000 },
      )
      .toBeGreaterThan(0)

    errors.assertClean('登录与仪表盘渲染')
  })

  test('未登录访问业务页被守卫拦到登录页（T0-1 守卫基线）', async ({ page }) => {
    // 清空会话：模拟直接输入 URL 的未登录访客
    await page.goto('/dashboard')
    await page.evaluate(() => localStorage.clear())

    await page.goto('/users')
    await expect(page).toHaveURL(/\/login$/)
  })
})
