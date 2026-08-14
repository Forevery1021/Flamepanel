import type { Directive } from 'vue'
import { STORAGE_KEYS } from '@/utils/storage'

type PermissionMode = 'manage' | 'view'

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
 */
export const permission: Directive<HTMLElement, string | { perm: string; mode?: PermissionMode }> = {
  mounted(el, binding) {
    apply(el, binding.value)
  },
  updated(el, binding) {
    apply(el, binding.value)
  },
}

function apply(
  el: HTMLElement,
  value: string | { perm: string; mode?: PermissionMode } | undefined,
) {
  const perm = typeof value === 'string' ? value : value?.perm
  const mode = typeof value === 'object' ? (value.mode ?? 'manage') : 'manage'
  if (!perm) return

  const role = localStorage.getItem(STORAGE_KEYS.role) || ''
  const allowed = hasPermission(role, perm)
  if (allowed) {
    el.style.removeProperty('opacity')
    el.style.removeProperty('pointer-events')
    el.removeAttribute('disabled')
    el.removeAttribute('aria-disabled')
    el.style.display = ''
    return
  }

  if (mode === 'view') {
    el.style.display = 'none'
    return
  }
  // manage 模式：置灰 + 阻断点击
  el.style.opacity = '0.5'
  el.style.pointerEvents = 'none'
  el.setAttribute('disabled', '')
  el.setAttribute('aria-disabled', 'true')
}

/** 判断角色是否拥有某权限（resource:action） */
export function hasPermission(role: string, perm: string): boolean {
  if (role === 'admin') return true
  const [, action] = perm.split(':')
  if (role === 'operator') {
    // operator 无 delete、无用户/设置/审计等管理类权限
    if (action === 'delete') return false
    if (perm.startsWith('user:') || perm.startsWith('settings:') || perm.startsWith('operation_log:')) {
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

/** 侧栏菜单角色可见性：返回该角色应隐藏的菜单（true=隐藏） */
export function isMenuHiddenForRole(role: string, path: string): boolean {
  if (role === 'admin' || !role) return false
  if (role === 'viewer') {
    // viewer 隐藏管理类菜单：用户、设置、Web 服务器（可写）、防火墙（可写）
    if (path === '/users' || path === '/settings') return true
  }
  if (role === 'operator') {
    // operator 隐藏纯管理：用户管理
    if (path === '/users') return true
  }
  return false
}
