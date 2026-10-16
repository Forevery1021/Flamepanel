import { test, expect } from '@playwright/test'
import { collectErrors, gotoMenu, login } from './helpers/flow'

/**
 * 黄金路径③：一个完整的 CRUD 闭环（备忘录 / 待办，Doc/23 §3.2 T1-2）。
 *
 * 为什么选 Memos：它是全仓唯一「创建 → 读回 → 更新 → 删除」四步都能在同一页完成
 * 且不依赖外部服务（Docker/MySQL/nginx）的模块，适合当 E2E 的冒烟闭环。
 *
 * 覆盖前端数据层与后端写路径的**真实往返**（axios → axum → SQLite → 回读渲染），
 * 以及 C-3 修复后的勾选语义（取消完成提交 done=false）在真实浏览器中的表现。
 */
test.describe('黄金路径③：备忘录 CRUD 闭环', () => {
  const todo = `E2E 待办-${Date.now()}`

  test('新增 → 勾选 → 取消勾选 → 删除', async ({ page }) => {
    const errors = collectErrors(page)
    await login(page)
    await gotoMenu(page, '/memos', '备忘录')

    const panel = page.locator('.p-tabpanel-active')
    const input = panel.locator('.memo-input input')
    const addBtn = panel.getByRole('button', { name: '添加' })

    // ── Create ──
    await input.fill(todo)
    await addBtn.click()
    const row = panel.locator('.memo-item').filter({ hasText: todo })
    await expect(row).toHaveCount(1)
    await expect(input, '创建成功后输入框应清空').toHaveValue('')
    // 新建项为未完成态
    await expect(row.locator('input[type="checkbox"]')).not.toBeChecked()

    // ── Update（勾选 → 取消勾选）──
    const checkbox = row.locator('input[type="checkbox"]')
    await checkbox.click()
    await expect(row).toHaveClass(/done/)
    await expect(checkbox).toBeChecked()

    await checkbox.click()
    await expect(row).not.toHaveClass(/done/)
    await expect(checkbox, '取消完成须真实生效（C-3 修复后的用户可见语义）').not.toBeChecked()

    // ── 刷新后仍在：证明写路径真的落了库（而非仅前端乐观更新）──
    await page.reload()
    const rowAfterReload = page.locator('.p-tabpanel-active .memo-item').filter({ hasText: todo })
    await expect(rowAfterReload).toHaveCount(1)

    // ── Delete（确认弹窗 → 行消失）──
    await rowAfterReload.locator('button').last().click()
    const dialog = page.locator('.p-confirmdialog')
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: '确定' }).click()
    await expect(rowAfterReload).toHaveCount(0, { timeout: 10_000 })

    // 刷新后确认删除已落库
    await page.reload()
    await expect(
      page.locator('.p-tabpanel-active .memo-item').filter({ hasText: todo }),
    ).toHaveCount(0)

    errors.assertClean('备忘录 CRUD 闭环')
  })
})
