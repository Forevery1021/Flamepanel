import type { Directive } from 'vue'

type PermissionMode = 'manage' | 'view'

/**
 * v-permission 权限指令（RBAC）：
 * 无权限时禁用（manage 模式：置灰不可点）或隐藏（view 模式）。
 * 用法：<FpButton v-permission="'docker:read'">…</FpButton>
 *       <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }">…</FpButton>
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

  const role = localStorage.getItem('role') || ''
  const allowed = role === 'admin' || hasPermission(role, perm)
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

/** operator 可操作运维类资源；viewer 只读 */
function hasPermission(role: string, perm: string): boolean {
  if (role === 'operator') {
    return !perm.startsWith('user') && !perm.startsWith('settings') && !perm.startsWith('operation_log')
  }
  return false
}
