import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { enableAutoUnmount, mount, flushPromises } from '@vue/test-utils'
import type { KernelLogEntry, KernelLogOverview, LogPage } from '@/api/kernelLogs'

/**
 * W0 内核日志数据层（`useKernelLogs`）。
 *
 * ① 首次刷新：概览 → 自动选中当前文件 → 第一页 + 五级计数；
 * ② 级别开关保持固定顺序并重拉；关键字防抖后重拉；
 * ③ 「加载更早」带 `before` 游标并追加；
 * ④ 跟随：文件大小不变不拉取，变化时只把新行合并到头部；
 * ⑤ 文件日志未启用：不发列表请求。
 */

const { api } = vi.hoisted(() => ({
  api: {
    getKernelLogOverview: vi.fn(),
    listKernelLogEntries: vi.fn(),
    getKernelLogStats: vi.fn(),
    downloadKernelLogFile: vi.fn(),
    deleteKernelLogFile: vi.fn(),
    setKernelLogLevel: vi.fn(),
  },
}))

vi.mock('@/api/kernelLogs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/kernelLogs')>()
  return { ...actual, ...api }
})

import {
  FOLLOW_INTERVAL_MS,
  PAGE_SIZE,
  SEARCH_DEBOUNCE_MS,
  useKernelLogs,
} from '@/composables/useKernelLogs'

const CUR = 'flamepanel.2026-09-25.log'
const OLD = 'flamepanel.2026-09-24.log'

function overview(size = 100, enabled = true): KernelLogOverview {
  return {
    enabled,
    dir: enabled ? '/opt/flamepanel/logs' : null,
    level: 'info',
    retention_days: 14,
    dropped_lines: 0,
    files: enabled
      ? [
          { name: CUR, size, modified: '2026-09-25T10:00:00Z', current: true },
          { name: OLD, size: 50, modified: '2026-09-24T23:59:59Z', current: false },
        ]
      : [],
  }
}

function entry(n: number, level: KernelLogEntry['level'] = 'info'): KernelLogEntry {
  return {
    timestamp: `2026-09-25T10:00:${String(n).padStart(2, '0')}Z`,
    level,
    target: 'flame_kernel',
    message: `m${n}`,
    fields: {},
  }
}

function page(ns: number[], opts: Partial<LogPage> = {}): { data: LogPage } {
  return {
    data: {
      file: CUR,
      entries: ns.map((n) => entry(n)),
      next_before: null,
      scanned_bytes: 100,
      file_size: 100,
      ...opts,
    },
  }
}

const counts = { trace: 0, debug: 0, info: 3, warn: 0, error: 0, total: 3 }

function setup(initial: { q?: string } = {}) {
  let state!: ReturnType<typeof useKernelLogs>
  const Host = defineComponent({
    setup() {
      state = useKernelLogs(initial)
      return () => h('div')
    },
  })
  const wrapper = mount(Host)
  return { state, wrapper }
}

enableAutoUnmount(afterEach)

describe('useKernelLogs', () => {
  beforeEach(() => {
    Object.values(api).forEach((f) => f.mockReset())
    api.getKernelLogOverview.mockResolvedValue({ data: overview() })
    api.listKernelLogEntries.mockResolvedValue(page([3, 2, 1]))
    api.getKernelLogStats.mockResolvedValue({ data: { file: CUR, counts, truncated: false } })
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('首次刷新：选中当前文件并拉取第一页与计数', async () => {
    const { state } = setup({ q: 'req-1' })
    await state.refresh()
    await flushPromises()
    expect(state.file.value).toBe(CUR)
    expect(state.isViewingCurrent.value).toBe(true)
    expect(api.listKernelLogEntries).toHaveBeenCalledWith(
      expect.objectContaining({ file: CUR, limit: PAGE_SIZE, q: 'req-1' }),
    )
    expect(state.rows.value.map((r) => r.message)).toEqual(['m3', 'm2', 'm1'])
    expect(new Set(state.rows.value.map((r) => r.key)).size).toBe(3)
    expect(state.counts.value?.total).toBe(3)
  })

  it('级别开关保持五级固定顺序，并以新条件重拉', async () => {
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    api.listKernelLogEntries.mockClear()
    state.toggleLevel('error')
    state.toggleLevel('warn')
    await flushPromises()
    expect(state.levels.value).toEqual(['warn', 'error'])
    expect(api.listKernelLogEntries).toHaveBeenLastCalledWith(
      expect.objectContaining({ levels: ['warn', 'error'] }),
    )
    state.toggleLevel('warn')
    await flushPromises()
    expect(state.levels.value).toEqual(['error'])
    state.clearLevels()
    await flushPromises()
    expect(state.levels.value).toEqual([])
  })

  it('关键字防抖：连续输入只触发一次请求', async () => {
    vi.useFakeTimers()
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    api.listKernelLogEntries.mockClear()
    state.keyword.value = 'd'
    await nextTick()
    state.keyword.value = 'db'
    await nextTick()
    state.keyword.value = 'db locked'
    await nextTick()
    expect(api.listKernelLogEntries).not.toHaveBeenCalled()
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS)
    await flushPromises()
    expect(api.listKernelLogEntries).toHaveBeenCalledTimes(1)
    expect(api.listKernelLogEntries).toHaveBeenCalledWith(
      expect.objectContaining({ q: 'db locked' }),
    )
  })

  it('加载更早：携带 before 游标并追加到末尾', async () => {
    api.listKernelLogEntries.mockResolvedValueOnce(page([9, 8], { next_before: 42 }))
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    expect(state.hasMore.value).toBe(true)
    api.listKernelLogEntries.mockResolvedValueOnce(page([7, 6]))
    await state.loadMore()
    expect(api.listKernelLogEntries).toHaveBeenLastCalledWith(
      expect.objectContaining({ before: 42 }),
    )
    expect(state.rows.value.map((r) => r.message)).toEqual(['m9', 'm8', 'm7', 'm6'])
    expect(state.hasMore.value).toBe(false)
    expect(new Set(state.rows.value.map((r) => r.key)).size).toBe(4)
  })

  it('跟随：大小不变不拉取；变大时只把新行合并到头部并标记为新', async () => {
    vi.useFakeTimers()
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    api.listKernelLogEntries.mockClear()

    state.following.value = true
    await flushPromises() // 开启即 tick 一次：大小未变
    expect(api.listKernelLogEntries).not.toHaveBeenCalled()

    api.getKernelLogOverview.mockResolvedValue({ data: overview(180) })
    api.listKernelLogEntries.mockResolvedValue(page([5, 4, 3, 2, 1], { file_size: 180 }))
    vi.advanceTimersByTime(FOLLOW_INTERVAL_MS)
    await flushPromises()
    expect(state.rows.value.map((r) => r.message)).toEqual(['m5', 'm4', 'm3', 'm2', 'm1'])
    expect(state.freshKeys.value.size).toBe(2)

    state.following.value = false
    await flushPromises()
    api.getKernelLogOverview.mockClear()
    vi.advanceTimersByTime(FOLLOW_INTERVAL_MS * 3)
    expect(api.getKernelLogOverview).not.toHaveBeenCalled()
  })

  it('切到历史文件：不再视为当前文件，并按新文件重拉', async () => {
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    api.listKernelLogEntries.mockClear()
    state.file.value = OLD
    await flushPromises()
    expect(state.isViewingCurrent.value).toBe(false)
    expect(api.listKernelLogEntries).toHaveBeenCalledWith(expect.objectContaining({ file: OLD }))
    expect(api.getKernelLogStats).toHaveBeenLastCalledWith(OLD)
  })

  it('文件日志未启用：不请求列表', async () => {
    api.getKernelLogOverview.mockResolvedValue({ data: overview(0, false) })
    const { state } = setup()
    await state.refresh()
    await flushPromises()
    expect(state.enabled.value).toBe(false)
    expect(api.listKernelLogEntries).not.toHaveBeenCalled()
    expect(state.rows.value).toEqual([])
  })

  it('调级：成功后同步到概览', async () => {
    api.setKernelLogLevel.mockResolvedValue({ data: { level: 'debug' } })
    const { state } = setup()
    await state.refresh()
    await expect(state.changeLevel('debug')).resolves.toBe('debug')
    expect(state.overview.value?.level).toBe('debug')
  })
})
