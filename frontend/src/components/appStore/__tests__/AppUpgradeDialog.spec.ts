import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const { api, toast } = vi.hoisted(() => ({
  api: { upgradeApp: vi.fn(), upgradePreview: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))
vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import AppUpgradeDialog from '@/components/appStore/AppUpgradeDialog.vue'

const messages = {
  appStore: {
    upgrade: '升级',
    upgradeTitle: '升级 {name}',
    upgradeSuccess: '升级成功',
    upgradeFailed: '升级失败',
    upgradeUpToDate: '已是最新',
    upgradeNoBase: '无基线',
    upgradeReformatted: '已重写',
    upgradeLocalKept: '保留本地（{n}）',
    upgradeUpstreamApplied: '上游修改（{n}）',
    upgradeConflicts: '冲突（{n}）',
    upgradePath: '路径',
    upgradeLocal: '本地',
    upgradeUpstream: '新版本',
    upgradeRemoved: '（已删除）',
    upgradeStrategy: '处理方式',
    upgradeKeepLocal: '保留本地修改',
    upgradeTakeUpstream: '采用新版本',
    upgradeNoChanges: '无差异',
    upgradePreviewFailed: '预览失败',
  },
  common: { cancel: '取消', retry: '重试', failed: '失败' },
}

const app = { id: 7, name: 'redis-cache', version: '7.2' } as never

function previewOf(merge: unknown, extra: Record<string, unknown> = {}) {
  return {
    data: {
      from_version: '7.2',
      to_version: '7.4',
      mode: 'container',
      up_to_date: false,
      base_available: true,
      merge,
      ...extra,
    },
  }
}
const cleanMerge = {
  merged: '',
  conflicts: [],
  local_changes: ['services.redis.ports[0]'],
  upstream_changes: ['services.redis.image'],
  reformatted: true,
}
const conflictMerge = {
  ...cleanMerge,
  conflicts: [{ path: 'services.redis.image', local: 'redis:7.2-alpine', upstream: 'redis:7.4' }],
  upstream_changes: [],
}

async function open() {
  document.body.innerHTML = ''
  const w = mount(AppUpgradeDialog, {
    props: { app: null },
    attachTo: document.body,
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  await w.setProps({ app })
  await flushPromises()
  await nextTick()
  return w
}
const text = () => document.body.textContent ?? ''
const upgradeBtn = () =>
  Array.from(document.body.querySelectorAll('button')).find(
    (b) => b.textContent?.trim() === '升级',
  ) as HTMLButtonElement

beforeEach(() => {
  vi.clearAllMocks()
  api.upgradeApp.mockResolvedValue({ data: {} })
})

describe('AppUpgradeDialog', () => {
  it('无冲突：展示版本与本地 / 上游修改，直接升级到预览目标版本', async () => {
    api.upgradePreview.mockResolvedValue(previewOf(cleanMerge))
    const w = await open()
    expect(api.upgradePreview).toHaveBeenCalledWith(7)
    expect(text()).toContain('升级 redis-cache')
    expect(text()).toContain('7.4')
    expect(text()).toContain('保留本地（1）')
    expect(text()).toContain('services.redis.ports[0]')
    expect(text()).toContain('上游修改（1）')
    expect(text()).toContain('已重写')
    expect(text()).not.toContain('无基线')
    expect(upgradeBtn().disabled).toBe(false)
    upgradeBtn().click()
    await flushPromises()
    expect(api.upgradeApp).toHaveBeenCalledWith(7, '7.4', undefined)
    expect(toast.success).toHaveBeenCalled()
    expect(w.emitted('done')).toHaveLength(1)
    expect(w.emitted('update:app')?.at(-1)).toEqual([null])
  })

  it('有冲突：必须选择处理方式，选择后按策略升级', async () => {
    api.upgradePreview.mockResolvedValue(previewOf(conflictMerge, { base_available: false }))
    await open()
    expect(text()).toContain('冲突（1）')
    expect(text()).toContain('redis:7.2-alpine')
    expect(text()).toContain('无基线')
    expect(upgradeBtn().disabled).toBe(true)
    const radios = Array.from(
      document.body.querySelectorAll('input[type="radio"]'),
    ) as HTMLInputElement[]
    expect(radios).toHaveLength(2)
    radios[1]!.click()
    await nextTick()
    expect(upgradeBtn().disabled).toBe(false)
    upgradeBtn().click()
    await flushPromises()
    expect(api.upgradeApp).toHaveBeenCalledWith(7, '7.4', 'upstream')
  })

  it('已是最新：禁用升级；预览失败显示错误', async () => {
    api.upgradePreview.mockResolvedValue(previewOf(null, { up_to_date: true, to_version: '7.2' }))
    await open()
    expect(text()).toContain('已是最新')
    expect(upgradeBtn().disabled).toBe(true)

    api.upgradePreview.mockRejectedValue(new Error('boom'))
    await open()
    expect(document.body.querySelector('[role="alert"]')).not.toBeNull()
    expect(upgradeBtn().disabled).toBe(true)
  })

  it('升级失败：提示错误并保持打开', async () => {
    api.upgradePreview.mockResolvedValue(previewOf(cleanMerge))
    api.upgradeApp.mockRejectedValue(new Error('409'))
    const w = await open()
    upgradeBtn().click()
    await flushPromises()
    expect(toast.error).toHaveBeenCalled()
    expect(w.emitted('done')).toBeUndefined()
  })
})
