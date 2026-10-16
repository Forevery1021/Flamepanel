/**
 * E2E 共享流程辅助（Doc/23 §3.2 T1-2）。
 *
 * 页面对象只有一层薄封装：**不加**自定义 `data-testid`（会污染生产模板），
 * 统一用「用户能看见的东西」定位——角色名 + 可见文案（i18n 译文）+ 稳定语义类名。
 * 译文取自 `src/locales/zh-CN.ts`（E2E 固定以 zh-CN 运行，见 playwright.config.ts）。
 */

import { expect, test, type Page } from '@playwright/test'

/** 向导创建的账号（唯一一次，串行用例共享） */
export const E2E_ADMIN = { username: 'e2eadmin', password: 'E2ePassw0rd!' }

/**
 * 登录次数预算（重要）：`/api/auth/login` 与 `/api/setup/initialize` 在后端共用
 * **登录档限流（5 次/分钟/IP）**。整套 E2E 为向导初始化 1 次 +「黄金路径②③」
 * 「04 新手引导」「05 设置」各登录 1 次，串行跑完不足一分钟，第 6 次会撞 429：
 * `login` 命中 429 时等窗口过期后重提一次（多耗约 1 分钟，不放宽生产限流）。
 * 因此首次提交可能留下一条 429 的 console 错误——需要零错误断言的用例请在
 * `login` **之后**再 `collectErrors`。新增用例仍应优先复用现有会话，勿无节制地登录。
 */

/** 控制台/页面错误采集：E2E 断言「黄金路径全程零未捕获错误」（R-2/R-3 的兜底网） */
export interface ErrorCollector {
  errors: string[]
  /** 断言期间无新增错误（可按段落调用） */
  assertClean: (stage: string) => void
}

export function collectErrors(page: Page): ErrorCollector {
  const errors: string[] = []
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console: ${m.text()}`)
  })
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`))
  return {
    errors,
    assertClean(stage: string) {
      expect(errors, `${stage} 期间出现未捕获错误`).toEqual([])
    },
  }
}

/** 黄金路径①：Setup 向导全流程（欢迎 → 管理员 → 数据库 → 服务器 → 主题 → 完成） */
export async function completeSetupWizard(page: Page): Promise<void> {
  await page.goto('/')
  await expect(page).toHaveURL(/\/setup$/)

  // 步骤 0 欢迎：环境探测（docker/nginx 不可用不影响继续）
  await page.getByRole('button', { name: '开始安装' }).click()

  // 步骤 1 管理员：密码须满足后端强度校验（≥8 位且至少两类字符）
  await page.getByLabel('管理员用户名').fill(E2E_ADMIN.username)
  await page.getByLabel('密码（至少 8 位）').fill(E2E_ADMIN.password)
  await page.getByLabel('确认密码').fill(E2E_ADMIN.password)

  // 步骤 2 数据库：SQLite 为默认项，无需额外输入
  await page.getByRole('button', { name: '下一步' }).click()
  await expect(page.locator('.step__title')).toHaveText('数据库配置')
  await expect(page.locator('.db-option.selected')).toContainText('SQLite')

  // 步骤 3 服务器：默认端口 8080 即可
  await page.getByRole('button', { name: '下一步' }).click()
  await expect(page.locator('.step__title')).toHaveText('服务器设置')

  // 步骤 4 主题：默认「烈焰」/ zh-CN
  await page.getByRole('button', { name: '下一步' }).click()
  await expect(page.locator('.step__title')).toHaveText('界面主题')

  // 步骤 5 完成：确认摘要后提交
  await page.getByRole('button', { name: '下一步' }).click()
  await expect(page.locator('.step__title')).toHaveText('准备完成')
  await expect(page.locator('.step-summary')).toContainText(E2E_ADMIN.username)

  await page.getByRole('button', { name: '开始安装' }).click()
  // 提交成功后**免二次登录**直接进入面板（T1-2 修复的缺陷即卡在此步）
  await expect(page).toHaveURL(/\/dashboard$/, { timeout: 20_000 })
}

/**
 * 跳过首次进入面板的新手引导（UX-2）。
 *
 * 每个 Playwright 上下文都是全新 localStorage，因此每次登录后都是「首次进入」，
 * 引导必然弹出；引导卡片会拦截其覆盖区域的点击（遮罩本身不吃事件），后续用例
 * 若不先关掉，点到卡片下方的按钮会一直重试到超时。这里显式等它出现再点「跳过」，
 * 顺带把「可跳过」这条产品约定放进真实浏览器里验证。
 */
export async function dismissOnboardingTour(page: Page): Promise<void> {
  const tour = page.getByRole('dialog', { name: '新手引导' })
  await tour.waitFor({ state: 'visible', timeout: 5_000 }).catch(() => undefined)
  if (await tour.isVisible()) {
    await tour.getByRole('button', { name: '跳过' }).click()
    await expect(tour).toBeHidden()
  }
}

/**
 * 登录（向导已完成后的复用入口；E2E 固定 zh-CN，走可见文案定位）。
 *
 * `keepTour: true` 时不自动跳过新手引导——`04-onboarding` 要亲手验证它的弹出与跳过。
 */
/** 登录档限流窗口（60 s）+ 余量 */
const LOGIN_WINDOW_MS = 62_000

export async function login(
  page: Page,
  user = E2E_ADMIN,
  opts: { keepTour?: boolean } = {},
): Promise<void> {
  await page.goto('/login')
  await page.getByLabel('用户名').fill(user.username)
  await page.getByLabel('密码', { exact: true }).fill(user.password)
  const submit = async () => {
    const [res] = await Promise.all([
      page.waitForResponse(
        (r) => r.url().endsWith('/api/auth/login') && r.request().method() === 'POST',
      ),
      page.getByRole('button', { name: /登\s*录/ }).click(),
    ])
    return res.status()
  }
  // 登录档限流 5 次 / 分钟（与 /api/setup/initialize 共用）：整套串行用例叠加后可能触顶，
  // 命中 429 时等窗口过期再提交一次，而不是放宽生产限流
  if ((await submit()) === 429) {
    test.info().setTimeout(test.info().timeout + LOGIN_WINDOW_MS)
    await page.waitForTimeout(LOGIN_WINDOW_MS)
    expect(await submit()).not.toBe(429)
  }
  await expect(page).toHaveURL(/\/dashboard$/, { timeout: 20_000 })
  if (!opts.keepTour) await dismissOnboardingTour(page)
}

/**
 * 经命令面板（⌘K / Ctrl+K）做一次**应用内**导航。
 *
 * `page.goto` 是整页加载，不经过 vue-router 的离开守卫；要验证「未保存离开确认」
 * 这类守卫必须走应用内跳转。命令面板是不受侧栏折叠状态影响的稳定入口。
 */
export async function navigateViaPalette(page: Page, query: string): Promise<void> {
  await page.keyboard.press('Control+k')
  const input = page.getByPlaceholder('搜索菜单或应用...')
  await expect(input).toBeVisible()
  await input.fill(query)
  await input.press('Enter')
}

/**
 * 进入某菜单页。
 *
 * 直接 `goto` 而非点侧栏：既覆盖「深链直达」路径（T0-1 路由守卫的同一条判定），
 * 又避免受侧栏分组折叠状态影响（分组展开是用户偏好，会被持久化）。
 */
export async function gotoMenu(page: Page, path: string, title: string): Promise<void> {
  await page.goto(path)
  await expect(page.locator('.lc-title__text')).toHaveText(title)
}
