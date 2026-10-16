import type { Directive, EffectScope, ShallowRef } from 'vue'
import { effectScope, shallowRef, watchEffect } from 'vue'
import { getActivePinia } from 'pinia'
import { useAuthStore } from '@/stores/auth'

type PermissionMode = 'manage' | 'view'

/** 指令绑定值：字符串（等价于默认 manage 模式）或对象（可指定 view 模式） */
type PermissionValue = string | { perm: string; mode?: PermissionMode }

/** 本指令可能写入的状态项（逐项记录，才能逐项精确还原） */
const STYLE_ITEMS = ['opacity', 'pointer-events', 'display'] as const
type StyleItem = (typeof STYLE_ITEMS)[number]
type WrittenItem = StyleItem | 'disabled' | 'aria-disabled'

/** 写入前的原值：null = 该项原本不存在（样式未设置 / 属性缺失） */
type OriginalValue = { value: string; priority: string } | null

/** 每个绑定元素的运行态：绑定值（响应式）+ 副作用作用域 + 写入账本 */
interface BindingState {
  bindingValue: ShallowRef<PermissionValue | undefined>
  scope: EffectScope
  /** 当前由**本指令**写入的项集合（不在集合内的项一律视为元素自持） */
  written: Set<WrittenItem>
  /** 各已写入项的元素自持基线（还原用；还原后随该项一并移除） */
  baseline: Map<WrittenItem, OriginalValue>
}

function isStyleItem(item: WrittenItem): item is StyleItem {
  return (STYLE_ITEMS as readonly string[]).includes(item)
}

/** 读取元素当前状态作为还原基线（不读指令写入项——调用方保证时机） */
function readItem(el: HTMLElement, item: WrittenItem): OriginalValue {
  if (isStyleItem(item)) {
    const value = el.style.getPropertyValue(item)
    return value ? { value, priority: el.style.getPropertyPriority(item) } : null
  }
  const value = el.getAttribute(item)
  return value === null ? null : { value, priority: '' }
}

/** 写入限制态 */
function writeItem(el: HTMLElement, item: WrittenItem, value: string): void {
  if (isStyleItem(item)) el.style.setProperty(item, value)
  else el.setAttribute(item, value)
}

/** 还原元素自持状态（基线为 null → 移除该项，绝不留下指令痕迹） */
function restoreItem(el: HTMLElement, item: WrittenItem, original: OriginalValue): void {
  if (isStyleItem(item)) {
    if (original) el.style.setProperty(item, original.value, original.priority)
    else el.style.removeProperty(item)
    return
  }
  if (original) el.setAttribute(item, original.value)
  else el.removeAttribute(item)
}

/** 元素 → 运行态。WeakMap：元素被回收后状态随之释放 */
const states = new WeakMap<HTMLElement, BindingState>()

/**
 * v-permission 权限指令（RBAC）：
 * 无权限时禁用（manage 模式：置灰不可点）或隐藏（view 模式）。
 * 用法：<FpButton v-permission="'docker:read'">…</FpButton>
 *       <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }">…</FpButton>
 *
 * 权限矩阵（与 Doc/07 一致）：
 * - admin：全部
 * - operator：除 delete 外的全部
 * - viewer：仅 read
 *
 * T0-2（Doc/23 §3.1 / 问题 P0-3、P1-5）——**角色单一来源 + 响应式重算**：
 * 判定依据一律取自 `useAuthStore()`（其 `role` 由 `useStorage` 承载，天然响应式），
 * 指令内**不再裸读 `localStorage`**。历史实现有两条缺口：
 *
 * 1. **读取双轨**：指令自行 `localStorage.getItem(STORAGE_KEYS.role)`，绕过 store
 *    （P1-5 存储双轨之一）——store 侧经 `/auth/me` 同步的角色、登出清空等状态变化
 *    对指令不可见；
 * 2. **非响应式**：仅在 `mounted` / `updated` 钩子判定，而 `updated` 的触发条件是
 *    「父组件重渲染」，与「角色变更」无关——切换账号或角色下发后，**已挂载的按钮**
 *    仍按旧角色渲染（只有重新挂载才生效）。
 *
 * 现在每个绑定元素持有一个 `effectScope` + `watchEffect`：effect 内同时读「绑定值」
 * 与「store 角色」两个依赖，任一变化即重新应用（幂等）；`unmounted` 停止作用域，
 * 多余的监听器不随分离节点泄漏。
 *
 * C 系列——**还原边界**：`restore` 必须只撤销「本指令写过的项」，**不得抹掉元素自身状态**。
 * 历史实现无条件 `removeAttribute('disabled' | 'aria-disabled')` 并清空
 * `opacity` / `pointer-events` / `display` 内联样式，于是**放行分支会把元素自身的禁用态
 * 一并撤销**：`<FpButton :disabled="item.installed" v-permission="…">` 在 admin 下渲染成
 * 可点击——RuntimeView 已安装版本的「安装」按钮即在生产环境丢失 disabled（本应置灰却可点）。
 * 现改为「写入账本 + 元素自持基线」：记下写入项与写入前的原值，还原时逐项恢复（原本不存在的
 * 项则移除），元素自身状态与其它指令/父组件的意图完整保留。
 *
 * **本指令只作用于「观感层」**（按钮是否可见/可点），不是安全边界——
 * 真正的拦截在每个 API 的服务端 RBAC（Doc/07 §3，默认拒绝）。
 */
export const permission: Directive<HTMLElement, PermissionValue> = {
  mounted(el, binding) {
    const state: BindingState = {
      bindingValue: shallowRef(binding.value),
      scope: effectScope(),
      written: new Set(),
      baseline: new Map(),
    }
    states.set(el, state)
    state.scope.run(() => {
      watchEffect(() => apply(el, state, state.bindingValue.value))
    })
  },
  updated(el, binding) {
    // 绑定值变化（如动态权限点）写回响应式引用，由 effect 统一重算——避免出现两条应用路径
    const state = states.get(el)
    if (state) state.bindingValue.value = binding.value
  },
  unmounted(el) {
    states.get(el)?.scope.stop()
    states.delete(el)
  },
}

/**
 * 解析当前角色——**单一来源** `useAuthStore()`。
 *
 * 指令钩子运行在组件树之外（`inject` 不可用），故经 `getActivePinia()` 取应用级
 * Pinia 实例（`app.use(pinia)` 时已 `setActivePinia`）。
 *
 * 无 Pinia（未装插件的极端场景 / 单测）→ 返回空角色：`hasPermission('', …)` 判定
 * 为无权限，与历史「本地无 role」行为一致（**fail-closed**：宁可少给按钮，不可越权放行）。
 */
export function resolveRole(): string {
  if (!getActivePinia()) return ''
  return useAuthStore().role || ''
}

/**
 * 应用权限态（幂等：同一状态下重复调用结果一致）。
 *
 * 允许 → 撤销本指令写过的项；拒绝 → 按模式隐藏（view）或置灰阻断（manage）。
 *
 * **写入账本**是这里的关键：`state.written` 记住「哪些项是本指令写的」，
 * `state.baseline` 记住「写之前元素自己是什么样」。还原时只撤销账本内的项、
 * 并恢复到基线值——**元素自持状态一律不碰**（根因见文件头 C 系列说明）。
 *
 * 统一走「先撤销不再需要的项，再写入目标项」两条路径，使 manage ↔ view 的模式
 * 切换也自然收敛：切到 view 时 manage 的置灰三件套会先被还原，不会叠加残留。
 */
function apply(el: HTMLElement, state: BindingState, value: PermissionValue | undefined): void {
  const perm = typeof value === 'string' ? value : value?.perm
  const mode = typeof value === 'object' ? (value?.mode ?? 'manage') : 'manage'
  const allowed = !perm || hasPermission(resolveRole(), perm)

  // 目标限制态：放行为空集；view = 仅隐藏；manage = 置灰 + 阻断 + 可访问性标记
  const target: Partial<Record<WrittenItem, string>> = allowed
    ? {}
    : mode === 'view'
      ? { display: 'none' }
      : { opacity: '0.5', 'pointer-events': 'none', disabled: '', 'aria-disabled': 'true' }

  // ① 撤销「本指令写过、本次不再需要」的项（含模式切换与放行两条路径）
  for (const item of [...state.written]) {
    if (item in target) continue
    restoreItem(el, item, state.baseline.get(item) ?? null)
    state.written.delete(item)
    state.baseline.delete(item)
  }

  // ② 写入目标项；首次写入前记录元素自持状态作为还原基线
  for (const item of Object.keys(target) as WrittenItem[]) {
    if (!state.written.has(item)) {
      state.baseline.set(item, readItem(el, item))
      state.written.add(item)
    }
    writeItem(el, item, target[item] as string)
  }
}

/** 判断角色是否拥有某权限（resource:action） */
export function hasPermission(role: string, perm: string): boolean {
  if (role === 'admin') return true
  const [, action] = perm.split(':')
  if (role === 'operator') {
    // operator 无 delete、无用户/设置/审计等管理类权限
    if (action === 'delete') return false
    if (
      perm.startsWith('user:') ||
      perm.startsWith('settings:') ||
      perm.startsWith('operation_log:')
    ) {
      return false
    }
    return true
  }
  if (role === 'viewer') {
    // viewer 仅 read（含 log/file/database 等只读资源）
    return action === 'read' && !perm.startsWith('user:')
  }
  return false
}

/**
 * 路由级权限判定（T0-1，Doc/23 §3.1）——守卫 / 侧栏 / ⌘K 命令面板**共用同一判据**。
 *
 * 历史缺口（P0-2）：判定只存在于侧栏 `isMenuHiddenForRole`，于是「菜单藏起来了，
 * 直接敲 URL 照样进」（观感越权），⌘K 命令面板也照旧列着无权限入口。
 *
 * 判据 = 页面 `meta.perm`（与后端 `api/permissions.rs` 的 resource:action 字面对齐）
 *      + 既有前端角色矩阵 `hasPermission`（admin 全量 / operator 除 delete 与管理类 /
 *        viewer 仅 read）。
 *
 * 语义边界（刻意如此，勿收紧）：
 * - `perm` 缺省 → 放行：用于无资源级权限点的落地页与自助页（`/dashboard` 是守卫的
 *   兜底重定向目标，若加判定会形成「重定向目标自身被拒」的循环；`/settings` 含
 *   仅需登录的自助改密，viewer 亦可用其只读页签；`/operation-logs` 后端对三种内置
 *   角色都放行 read）；
 * - **未知角色（非 admin/operator/viewer 的自定义角色）→ 放行**：前端只能表达内置
 *   三角色矩阵，对自定义角色无从判定；误锁代价（整站不可用）远高于观感收益；
 * - 角色为空 → 放行：`/auth/me` 尚未把服务端角色同步到本地时的过渡态。
 *
 * **本函数只解决「页面框架是否渲染」的观感问题，不承担安全边界**——
 * 真正的拦截在每个 API 的服务端 RBAC（Doc/07 §3，默认拒绝）。
 */
export function hasRoutePermission(role: string, meta: { perm?: string } | undefined): boolean {
  const perm = meta?.perm
  if (!perm) return true
  if (!isKnownRole(role)) return true
  return hasPermission(role, perm)
}

/** 三种内置角色（前端矩阵可表达的范围） */
function isKnownRole(role: string): boolean {
  return role === 'admin' || role === 'operator' || role === 'viewer'
}

/**
 * 菜单项可见性（侧栏 / ⌘K 命令面板）——在权限判定之上叠加既有的**角色级菜单规则**
 * `isMenuHiddenForRole`（历史语义：viewer/operator 隐藏 `/users`、viewer 隐藏 `/settings`）。
 *
 * 为什么菜单比守卫多一条规则：菜单是「主动推荐入口」，比「URL 可直达」更保守；
 * 反之则必须避免——**菜单可见但守卫拒绝**会形成死路（点了就被弹回）。
 */
export function isRouteVisibleInMenu(
  role: string,
  route: { path: string; meta?: { perm?: string } | undefined },
): boolean {
  if (!isKnownRole(role)) return true
  if (isMenuHiddenForRole(role, route.path)) return false
  return hasRoutePermission(role, route.meta)
}

/** 侧栏菜单角色可见性：返回该角色应隐藏的菜单（true=隐藏） */
export function isMenuHiddenForRole(role: string, path: string): boolean {
  if (role === 'admin' || !role) return false
  if (role === 'viewer') {
    // viewer 隐藏管理类菜单：用户、设置、Web 服务器（可写）、防火墙（可写）
    if (path === '/users' || path === '/settings') return true
    // 终端为 write 能力面（terminal:execute，viewer 无此权限）——菜单必须同步隐藏，
    // 否则「菜单可见 → 点进去被守卫弹回」形成死路（T0-1 一致性要求）
    if (path === '/terminal') return true
  }
  if (role === 'operator') {
    // operator 隐藏纯管理：用户管理
    if (path === '/users') return true
  }
  return false
}
