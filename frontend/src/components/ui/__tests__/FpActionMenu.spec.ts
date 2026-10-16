import { describe, it, expect, vi } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { mountFp, createTestI18n, openVuePlugin, services } from './helpers'
import FpActionMenu, { type FpActionItem } from '../FpActionMenu.vue'
import { useAuthStore } from '@/stores/auth'

/**
 * FpActionMenu：行操作「更多」菜单。
 * 契约：perm 过滤与 v-permission 同源（无权限项不渲染）；danger 项前自动分隔；
 * disabled/loading 项置灰；无可见项时触发按钮禁用；点击项调用 command。
 */
function items(over: Partial<FpActionItem>[] = []): FpActionItem[] {
  const base: FpActionItem[] = [
    { key: 'restart', label: '重启', icon: 'oi oi-sync', command: vi.fn() },
    { key: 'rename', label: '重命名', perm: 'docker:update', command: vi.fn() },
    { key: 'remove', label: '删除', perm: 'docker:delete', danger: true, command: vi.fn() },
  ]
  return base.map((b, i) => ({ ...b, ...(over[i] ?? {}) }))
}

async function open(w: ReturnType<typeof mountFp>) {
  await w.find('button').trigger('click')
  await nextTick()
  await nextTick()
  return Array.from(
    document.body.querySelectorAll('.fp-action-menu .fp-action-menu__item'),
  ) as HTMLElement[]
}

function withRole(role: string) {
  const pinia = createPinia()
  setActivePinia(pinia)
  useAuthStore().role = role
  return pinia
}

describe('FpActionMenu 行操作菜单', () => {
  it('admin：全部项可见，删除项为 danger 且前有分隔线', async () => {
    const pinia = withRole('admin')
    const w = mountFp(FpActionMenu, {
      props: { items: items() },
      global: {
        plugins: [
          createTestI18n({ common: { more: '更多操作' } }),
          openVuePlugin,
          ...services,
          pinia,
        ],
      },
      attachTo: document.body,
    })
    const els = await open(w)
    expect(els.map((e) => e.textContent?.trim())).toEqual(['重启', '重命名', '删除'])
    expect(els[2].classList.contains('fp-action-menu__item--danger')).toBe(true)
    expect(document.body.querySelectorAll('.fp-action-menu .p-tieredmenu-separator').length).toBe(1)
    w.unmount()
  })

  it('viewer：带 perm 的项不渲染（与 v-permission view 模式同源，fail-closed）', async () => {
    const pinia = withRole('viewer')
    const w = mountFp(FpActionMenu, {
      props: { items: items() },
      global: {
        plugins: [
          createTestI18n({ common: { more: '更多操作' } }),
          openVuePlugin,
          ...services,
          pinia,
        ],
      },
      attachTo: document.body,
    })
    const els = await open(w)
    expect(els.map((e) => e.textContent?.trim())).toEqual(['重启'])
    w.unmount()
  })

  it('operator：可 update 不可 delete', async () => {
    const pinia = withRole('operator')
    const w = mountFp(FpActionMenu, {
      props: { items: items() },
      global: {
        plugins: [
          createTestI18n({ common: { more: '更多操作' } }),
          openVuePlugin,
          ...services,
          pinia,
        ],
      },
      attachTo: document.body,
    })
    const els = await open(w)
    expect(els.map((e) => e.textContent?.trim())).toEqual(['重启', '重命名'])
    w.unmount()
  })

  it('点击项调用其 command；disabled 项置灰', async () => {
    const pinia = withRole('admin')
    const list = items([{}, { disabled: true }])
    const w = mountFp(FpActionMenu, {
      props: { items: list },
      global: {
        plugins: [
          createTestI18n({ common: { more: '更多操作' } }),
          openVuePlugin,
          ...services,
          pinia,
        ],
      },
      attachTo: document.body,
    })
    const els = await open(w)
    els[0].click()
    expect(list[0].command).toHaveBeenCalledTimes(1)
    expect(els[1].classList.contains('p-disabled')).toBe(true)
    w.unmount()
  })

  it('无可见项：触发按钮禁用', () => {
    const pinia = withRole('viewer')
    const only = [{ key: 'x', label: 'X', perm: 'docker:delete', command: vi.fn() }]
    const w = mountFp(FpActionMenu, {
      props: { items: only },
      global: {
        plugins: [
          createTestI18n({ common: { more: '更多操作' } }),
          openVuePlugin,
          ...services,
          pinia,
        ],
      },
    })
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })
})
