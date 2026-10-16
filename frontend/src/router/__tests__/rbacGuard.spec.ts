import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent } from 'vue'
import { STORAGE_KEYS } from '@/utils/storage'
import { resetNoticeChannel, onNotice, type NoticeMessage } from '@/utils/notice'
import { routerGuard, checkPagePermission } from '@/router'

/**
 * T0-1 路由级 RBAC 守卫（Doc/23 §3.1 / 问题 P0-2）集成回归。
 *
 * 用真实 router + 真实守卫（`routerGuard`）驱动导航，断言：
 * 1. **viewer 直接输入 `/users` URL 被重定向 `/dashboard`**（历史缺口：页面框架照常渲染）；
 * 2. admin 不受影响（同一条 URL 正常进入）；
 * 3. 拒绝时投递一次 i18n-key 形式的提示（不固化译文——守卫侧语言包可能未就绪）；
 * 4. 放行页（`/dashboard`）与登录守卫（未登录 → `/login`）行为保持。
 *
 * 说明：`/setup/status` 探测在测试中桩为空态，避免真实网络请求。
 */

vi.mock('@/api/setup', () => ({
  getSetupStatus: vi.fn().mockResolvedValue({ data: { status: 'completed' } }),
  initializeSetup: vi.fn(),
}))

/** 极简页面组件（不渲染内容，只关心路由落点） */
const Page = defineComponent({ name: 'StubPage', render: () => null })

function buildRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', name: 'Login', component: Page },
      { path: '/setup', name: 'Setup', component: Page },
      { path: '/dashboard', name: 'Dashboard', component: Page, meta: { title: 'nav.dashboard' } },
      {
        path: '/users',
        name: 'Users',
        component: Page,
        meta: { title: 'nav.users', perm: 'user:read' },
      },
      {
        path: '/nodes',
        name: 'Nodes',
        component: Page,
        meta: { title: 'nav.nodes', perm: 'node:read' },
      },
      {
        path: '/terminal',
        name: 'Terminal',
        component: Page,
        meta: { title: 'nav.terminal', perm: 'terminal:execute' },
      },
    ],
  })
  router.beforeEach(routerGuard)
  return router
}

/** 以指定角色登录（token + role 写入 localStorage，与 auth store 持久化格式一致） */
function loginAs(role: string) {
  localStorage.setItem(STORAGE_KEYS.token, 'test-token')
  localStorage.setItem(STORAGE_KEYS.username, 'tester')
  localStorage.setItem(STORAGE_KEYS.role, role)
  setActivePinia(createPinia())
}

describe('T0-1 路由级 RBAC 守卫', () => {
  let notices: NoticeMessage[]

  beforeEach(() => {
    localStorage.clear()
    resetNoticeChannel()
    notices = []
    onNotice((m) => notices.push(m))
  })

  it('【关键回归】viewer 直接访问 /users → 重定向 /dashboard 并给出提示', async () => {
    loginAs('viewer')
    const router = buildRouter()
    await router.push('/users')
    await router.isReady()
    expect(router.currentRoute.value.path).toBe('/dashboard')
    // 提示为 i18n key 形式（取词交给订阅方，避免守卫侧语言包未就绪时上屏裸 key）
    expect(notices).toEqual([{ key: 'common.noPermission', severity: 'warn' }])
  })

  it('【关键回归】viewer 访问 write 能力面 /terminal → 同样被拦', async () => {
    loginAs('viewer')
    const router = buildRouter()
    await router.push('/terminal')
    expect(router.currentRoute.value.path).toBe('/dashboard')
    expect(notices).toHaveLength(1)
  })

  it('admin 不受影响：同一 URL 正常进入', async () => {
    loginAs('admin')
    const router = buildRouter()
    await router.push('/users')
    expect(router.currentRoute.value.path).toBe('/users')
    expect(notices).toHaveLength(0)
  })

  it('operator 可进读页面、不可进用户管理', async () => {
    loginAs('operator')
    const router = buildRouter()
    await router.push('/nodes')
    expect(router.currentRoute.value.path).toBe('/nodes')

    await router.push('/users')
    expect(router.currentRoute.value.path).toBe('/dashboard')
  })

  it('viewer 可进入读页面 /nodes（RBAC 只拦越权，不误伤只读）', async () => {
    loginAs('viewer')
    const router = buildRouter()
    await router.push('/nodes')
    expect(router.currentRoute.value.path).toBe('/nodes')
    expect(notices).toHaveLength(0)
  })

  it('未登录仍走登录跳转（原有行为不回退）', async () => {
    setActivePinia(createPinia())
    const router = buildRouter()
    await router.push('/nodes')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('重定向目标 /dashboard 自身不被权限判定拦截（防自环）', async () => {
    loginAs('viewer')
    const router = buildRouter()
    await router.push('/dashboard')
    expect(router.currentRoute.value.path).toBe('/dashboard')
    expect(notices).toHaveLength(0)
  })

  it('checkPagePermission 在目标页即 /dashboard 时放行（自环兜底）', () => {
    loginAs('viewer')
    expect(
      checkPagePermission({
        path: '/dashboard',
        meta: { perm: 'user:read' },
      } as never),
    ).toBe(true)
  })

  it('无 meta.perm 的页面放行', () => {
    loginAs('viewer')
    expect(checkPagePermission({ path: '/dashboard', meta: {} } as never)).toBe(true)
  })
})
