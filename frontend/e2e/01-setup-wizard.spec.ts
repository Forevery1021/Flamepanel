import { test, expect } from '@playwright/test'
import { E2E_ADMIN, collectErrors, completeSetupWizard } from './helpers/flow'

/**
 * 黄金路径①：Setup 向导全流程 → 免二次登录进入面板（Doc/23 §3.2 T1-2）。
 *
 * 这是**唯一**必须在「全新库」上跑的用例（向导只在空库上可走通一次），故文件名
 * 前缀 01 让它先执行（配置为串行，见 playwright.config.ts）。它跑完后库即处于
 * completed 状态，后继用例复用同一后端。
 *
 * 覆盖的不只是「向导能点完」——提交后**免二次登录直达面板**是向导的既有承诺
 * （`SetupView.handleSubmit` 保存后端下发的令牌后 `router.push('/dashboard')`）。
 * 本用例正是靠这条断言，抓出了「初始化状态被 store 过期缓存拦回 /setup」的缺陷；
 * 而提交后的深链/刷新断言，锁的是同一缓存与后端真值的一致性。
 */
test.describe('黄金路径①：Setup 向导', () => {
  test('空库首访落在向导页，六步走完直达面板，且刷新/深链不被拦回向导', async ({ page }) => {
    const errors = collectErrors(page)

    // 前置校验：**经前端源**访问后端必须到达本次 global-setup 启的那个实例。
    // 若本地残留了指向别的后端的 preview 进程（`reuseExistingServer` 会复用它），
    // 整套 E2E 会在错误的实例上跑出「假绿」——用全新库的 in_progress 状态把这条路堵死。
    const proxied = await page.request.get('/api/setup/status')
    expect(proxied.ok(), '前端 /api 反代未生效（检查 VITE_PROXY_TARGET）').toBe(true)
    expect(((await proxied.json()) as { status: string }).status).toBe('in_progress')

    // 未初始化：根路径被守卫导向 /setup
    await page.goto('/')
    await expect(page).toHaveURL(/\/setup$/)
    // 步骤指示器六步齐全（欢迎/管理员/数据库/服务器/主题/完成）
    await expect(page.locator('.setup-step')).toHaveCount(6)

    // 续填（S-14）：走到数据库步后刷新 → 回到管理员步补填密码；用户名回填、密码不落盘
    await page.getByRole('button', { name: '开始安装' }).click()
    await page.getByLabel('管理员用户名').fill(E2E_ADMIN.username)
    await page.getByLabel('密码（至少 8 位）').fill(E2E_ADMIN.password)
    await page.getByLabel('确认密码').fill(E2E_ADMIN.password)
    await page.getByRole('button', { name: '下一步' }).click()
    await expect(page.locator('.step__title')).toHaveText('数据库配置')
    await page.reload()
    await expect(
      page.getByRole('status').filter({ hasText: '已恢复上次未完成的安装进度' }),
    ).toBeVisible()
    await expect(page.getByLabel('管理员用户名')).toHaveValue(E2E_ADMIN.username)
    await expect(page.getByLabel('密码（至少 8 位）')).toHaveValue('')
    await expect(page.getByLabel('确认密码')).toHaveValue('')
    const stored = await page.evaluate(() => JSON.stringify({ ...sessionStorage }))
    expect(stored, '草稿不得持久化密码').not.toContain(E2E_ADMIN.password)
    // 「重新开始」清掉草稿，回到欢迎步——下面的完整向导从头走
    await page.getByRole('button', { name: '重新开始' }).click()
    await expect(page.getByRole('button', { name: '开始安装' })).toBeVisible()

    await completeSetupWizard(page)

    // 免二次登录：localStorage 已持有后端签发的令牌与服务端确认的角色
    const auth = await page.evaluate(() => ({
      token: localStorage.getItem('token'),
      username: localStorage.getItem('username'),
      role: localStorage.getItem('role'),
    }))
    expect(auth.token, '向导提交后应持有 access token（免二次登录）').toBeTruthy()
    expect(auth.username).toBe(E2E_ADMIN.username)
    expect(auth.role, '角色须来自服务端下发，而非前端推测').toBe('admin')

    // 后端状态确已翻转为 completed（与前端判定同源）
    const status = await page.evaluate(async () => {
      const res = await fetch('/api/setup/status')
      return ((await res.json()) as { status: string }).status
    })
    expect(status).toBe('completed')

    // 深链 + 刷新：状态判定不得再把已登录用户送回向导。
    // （修复前这里读的是 store 里过期的 in_progress，刷新也不能自愈。）
    await page.goto('/memos')
    await page.reload()
    await expect(page).toHaveURL(/\/memos$/)
    await expect(page.locator('.lc-title__text')).toHaveText('备忘录')

    errors.assertClean('向导全流程与完成后的深链/刷新')
  })
})
