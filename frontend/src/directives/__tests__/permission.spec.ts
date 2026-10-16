import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h, nextTick, ref, withDirectives } from 'vue'
import { hasPermission, isMenuHiddenForRole, permission } from '@/directives/permission'
import { useAuthStore } from '@/stores/auth'
import { STORAGE_KEYS } from '@/utils/storage'

// 隔离网络：`auth.logout()` 会顺带调后端吊销 refresh token——单测中不打真实请求
vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  logout: vi.fn().mockResolvedValue({ data: { message: 'ok' } }),
  refreshToken: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

describe('hasPermission（RBAC 权限矩阵，与 Doc/07 一致）', () => {
  it('admin 拥有全部权限（含 delete 与管理类）', () => {
    expect(hasPermission('admin', 'docker:delete')).toBe(true)
    expect(hasPermission('admin', 'user:delete')).toBe(true)
    expect(hasPermission('admin', 'settings:write')).toBe(true)
    expect(hasPermission('admin', 'node:read')).toBe(true)
  })

  it('operator 无 delete 权限', () => {
    expect(hasPermission('operator', 'docker:start')).toBe(true)
    expect(hasPermission('operator', 'docker:read')).toBe(true)
    expect(hasPermission('operator', 'docker:delete')).toBe(false)
    expect(hasPermission('operator', 'node:delete')).toBe(false)
  })

  it('operator 无用户/设置/审计管理类权限', () => {
    expect(hasPermission('operator', 'user:read')).toBe(false)
    expect(hasPermission('operator', 'settings:write')).toBe(false)
    expect(hasPermission('operator', 'operation_log:read')).toBe(false)
  })

  it('viewer 仅 read 且不含 user 资源', () => {
    expect(hasPermission('viewer', 'docker:read')).toBe(true)
    expect(hasPermission('viewer', 'log:read')).toBe(true)
    expect(hasPermission('viewer', 'docker:start')).toBe(false)
    expect(hasPermission('viewer', 'file:delete')).toBe(false)
    expect(hasPermission('viewer', 'user:read')).toBe(false)
  })

  it('未知角色或空角色一律拒绝', () => {
    expect(hasPermission('', 'docker:read')).toBe(false)
    expect(hasPermission('unknown', 'docker:read')).toBe(false)
  })
})

describe('isMenuHiddenForRole（侧栏菜单角色可见性）', () => {
  it('admin / 空角色不隐藏任何菜单', () => {
    expect(isMenuHiddenForRole('admin', '/users')).toBe(false)
    expect(isMenuHiddenForRole('', '/users')).toBe(false)
  })

  it('viewer 隐藏用户与设置菜单', () => {
    expect(isMenuHiddenForRole('viewer', '/users')).toBe(true)
    expect(isMenuHiddenForRole('viewer', '/settings')).toBe(true)
    expect(isMenuHiddenForRole('viewer', '/nodes')).toBe(false)
  })

  it('operator 隐藏用户菜单', () => {
    expect(isMenuHiddenForRole('operator', '/users')).toBe(true)
    expect(isMenuHiddenForRole('operator', '/nodes')).toBe(false)
  })
})

/* ═══════════ T0-2：指令响应式化 + 角色单一来源（Doc/23 §3.1 / P0-3、P1-5） ═══════════
 *
 * 历史实现的两个缺口（本组用例逐条机器化锁定）：
 * 1. **读取双轨**：指令自行 `localStorage.getItem(STORAGE_KEYS.role)`，绕过 auth store，
 *    篡改本地存储即可解锁按钮（store 侧的角色变化对指令不可见）；
 * 2. **非响应式**：只在 `mounted` / `updated` 判定，而 `updated` 触发条件是父组件重渲染，
 *    与角色变更无关——切换账号/角色下发后已挂载按钮仍按旧角色渲染。
 */

type DirectiveValue = string | { perm: string; mode?: 'manage' | 'view' }

/**
 * 指令宿主工厂——全文件**单处** `defineComponent` 调用（避免 spec 内堆叠组件定义触发
 * `vue/one-component-per-file`，与 FpErrorBoundary.spec.ts 同一约定）。
 * `value` 取 getter 形式：测试内可闭包持有 `ref`，驱动「绑定值变化」场景。
 */
const Host = (
  value: () => DirectiveValue,
  classes = 'target',
  attrs: Record<string, unknown> = {},
) =>
  defineComponent({
    name: 'PermissionHost',
    setup: () => () =>
      withDirectives(h('button', { class: classes, ...attrs }, '操作'), [[permission, value()]]),
  })

/** 挂载宿主并等待 effect 首次执行；`attrs` 用于复现「元素自身已有属性」的场景 */
async function mountHost(value: DirectiveValue, attrs: Record<string, unknown> = {}) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const w = mount(
    Host(() => value, 'target', attrs),
    { global: { plugins: [pinia] } },
  )
  await nextTick()
  return { w, auth: useAuthStore() }
}

const btn = (w: Awaited<ReturnType<typeof mountHost>>['w']) => w.find('button')

describe('v-permission 指令（T0-2 响应式化）', () => {
  beforeEach(() => {
    localStorage.clear()
  })
  afterEach(() => {
    localStorage.clear()
  })

  it('【关键回归】判定角色取自 auth store，而非裸读 localStorage', async () => {
    const { w, auth } = await mountHost('docker:start')
    // store 侧角色 = viewer（无权 docker:start）
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()

    // 直接篡改本地存储为 admin（绕过 store）——指令不得据此放行
    localStorage.setItem(STORAGE_KEYS.role, 'admin')
    await nextTick()
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()
    expect(btn(w).attributes('aria-disabled')).toBe('true')

    // 反向验证：经 store 同步为 admin 后才放行（单一来源 = store）
    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeUndefined()
  })

  it('【关键回归】已挂载按钮随角色变更实时更新（无需重新挂载）', async () => {
    const { w, auth } = await mountHost('docker:start')
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()

    // 角色下放：viewer → admin（切换账号 / `/auth/me` 同步场景）
    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeUndefined()
    expect(btn(w).attributes('aria-disabled')).toBeUndefined()

    // 角色回收：admin → viewer，按钮应重新置灰
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()
  })

  it('登出清空角色 → 权限收紧（不回退为放行）', async () => {
    const { w, auth } = await mountHost('docker:start')
    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeUndefined()

    auth.logout()
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()
  })

  it('manage 模式：置灰 + 阻断点击 + aria-disabled；有权限时完整还原', async () => {
    const { w, auth } = await mountHost({ perm: 'docker:delete' })
    auth.role = 'operator' // operator 无 delete
    await nextTick()
    expect(btn(w).attributes('style')).toContain('opacity')
    expect(btn(w).attributes('style')).toContain('pointer-events: none')
    expect(btn(w).attributes('disabled')).toBeDefined()
    expect(btn(w).attributes('aria-disabled')).toBe('true')

    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('style') ?? '').not.toContain('opacity')
    expect(btn(w).attributes('style') ?? '').not.toContain('pointer-events')
    expect(btn(w).attributes('disabled')).toBeUndefined()
    expect(btn(w).attributes('aria-disabled')).toBeUndefined()
  })

  it('view 模式：无权限时隐藏（display:none），有权限时恢复显示', async () => {
    const { w, auth } = await mountHost({ perm: 'docker:delete', mode: 'view' })
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('style')).toContain('display: none')

    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('style') ?? '').not.toContain('display: none')
    // view 模式不应顺手加上 manage 的置灰态
    expect(btn(w).attributes('disabled')).toBeUndefined()
  })

  it('空权限点 / 未传值 → 还原（不做任何限制）', async () => {
    const { w, auth } = await mountHost('')
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeUndefined()
    expect(btn(w).attributes('style') ?? '').not.toContain('display: none')
  })

  it('角色未同步（空角色）→ 视为无权限，不越权放行', async () => {
    const { w, auth } = await mountHost('docker:start')
    auth.role = ''
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()
  })

  it('未安装 Pinia 时降级为空角色（fail-closed，不抛异常）', async () => {
    setActivePinia(undefined)
    const w = mount(Host(() => 'docker:start'))
    await nextTick()
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })

  it('指令绑定值变化时按新权限点重算（updated 钩子写回响应式引用）', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const auth = useAuthStore()
    auth.role = 'viewer'

    const perm = ref<DirectiveValue>('docker:read')
    const w = mount(
      Host(() => perm.value, 'dynamic'),
      { global: { plugins: [pinia] } },
    )
    await nextTick()
    // viewer 有 docker:read → 放行
    expect(w.find('button').attributes('disabled')).toBeUndefined()

    // 切换到 viewer 无权的权限点 → 收紧（绑定值经 updated 钩子写回响应式引用后重算）
    perm.value = 'docker:delete'
    await nextTick()
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })

  /* ───────── C 系列：还原边界——「放行」不得抹掉元素自身状态 ─────────
   *
   * 根因：`restore()` 无条件 `removeAttribute('disabled')` / `removeAttribute('aria-disabled')`
   * 并清空 opacity / pointer-events / display 内联样式——放行分支会顺手抹掉**元素自身**
   * 的禁用态与内联样式：`<FpButton :disabled="item.installed" v-permission=…>` 在 admin
   * 下渲染成**可点击**（RuntimeView 已安装版本的「安装」按钮即受害者）。
   *
   * 语义修正：**只还原本指令写过的项**，元素自身状态一律不碰。
   */
  it('【关键回归】元素自身已 disabled：放行时不抹除（RuntimeView 场景）', async () => {
    const { w, auth } = await mountHost('runtime:create', { disabled: true })
    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()
    expect((btn(w).element as HTMLButtonElement).disabled).toBe(true)
  })

  it('【关键回归】denied → allowed 后元素自身 disabled 仍在', async () => {
    const { w, auth } = await mountHost('docker:start', { disabled: true })
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()

    auth.role = 'admin'
    await nextTick()
    // 元素自身的禁用态属于父组件意图，不得随指令一起撤销
    expect((btn(w).element as HTMLButtonElement).disabled).toBe(true)
  })

  it('原本未禁用的元素：指令写入的 disabled 在放行后移除（不残留）', async () => {
    const { w, auth } = await mountHost('docker:start')
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeDefined()

    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('disabled')).toBeUndefined()
  })

  it('元素自身内联 opacity 在置灰 → 放行后按原值恢复（不是清空）', async () => {
    const { w, auth } = await mountHost('docker:start', { style: 'opacity: 0.3' })
    auth.role = 'viewer'
    await nextTick()
    expect((btn(w).element as HTMLElement).style.opacity).toBe('0.5')

    auth.role = 'admin'
    await nextTick()
    expect((btn(w).element as HTMLElement).style.opacity).toBe('0.3')
  })

  it('元素自身 aria-disabled="false" 在置灰 → 放行后按原值恢复', async () => {
    const { w, auth } = await mountHost('docker:start', { 'aria-disabled': 'false' })
    auth.role = 'viewer'
    await nextTick()
    expect(btn(w).attributes('aria-disabled')).toBe('true')

    auth.role = 'admin'
    await nextTick()
    expect(btn(w).attributes('aria-disabled')).toBe('false')
  })

  it('view 模式：元素自身 display 原值在放行后恢复', async () => {
    const { w, auth } = await mountHost(
      { perm: 'docker:delete', mode: 'view' },
      { style: 'display: inline-flex' },
    )
    auth.role = 'viewer'
    await nextTick()
    expect((btn(w).element as HTMLElement).style.display).toBe('none')

    auth.role = 'admin'
    await nextTick()
    expect((btn(w).element as HTMLElement).style.display).toBe('inline-flex')
  })

  it('元素卸载后角色变更不再影响已分离节点（effectScope 已停止）', async () => {
    const { w, auth } = await mountHost('docker:start')
    auth.role = 'admin'
    await nextTick()
    const el = btn(w).element as HTMLButtonElement
    w.unmount()

    auth.role = 'viewer'
    await nextTick()
    await nextTick()
    // 分离节点不应再被写入禁用态（作用域已随 unmounted 停止）
    expect(el.hasAttribute('disabled')).toBe(false)
  })
})
