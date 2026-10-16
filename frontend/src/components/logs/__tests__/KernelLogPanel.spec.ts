import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { enableAutoUnmount, flushPromises } from '@vue/test-utils'
import { mountFp } from '@/components/ui/__tests__/helpers'
import zhLogs from '@/locales/zh-CN/logs'
import type { KernelLogEntry, KernelLogOverview } from '@/api/kernelLogs'

/**
 * W0 内核日志面板交互契约：
 * ① 五级开关：aria-pressed 反映选中态，显示各级计数；
 * ② 行点击展开详情：结构化字段 + 请求编号，「只看此请求」把编号填入搜索；
 * ③ 关键字在消息中高亮（纯文本分段，无 v-html）；
 * ④ 当前文件不可删除（按钮置灰）；未启用文件日志时给出配置指引。
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

import KernelLogPanel from '@/components/logs/KernelLogPanel.vue'

const CUR = 'flamepanel.2026-09-25.log'

function overview(enabled = true): KernelLogOverview {
  return {
    enabled,
    dir: enabled ? '/opt/flamepanel/logs' : null,
    level: 'info',
    retention_days: 14,
    dropped_lines: 0,
    files: enabled
      ? [{ name: CUR, size: 2048, modified: '2026-09-25T10:00:00Z', current: true }]
      : [],
  }
}

const entries: KernelLogEntry[] = [
  {
    timestamp: '2026-09-25T10:00:02Z',
    level: 'error',
    target: 'flame_kernel::core::error',
    message: 'Internal server error: db locked',
    request_id: 'req-abc',
    fields: { error: 'db locked' },
  },
  {
    timestamp: '2026-09-25T10:00:01Z',
    level: 'info',
    target: 'flame_kernel',
    message: 'kernel starting',
    fields: {},
  },
]

function mountPanel(props: Record<string, unknown> = {}) {
  return mountFp(KernelLogPanel, {
    props,
    messages: { log: zhLogs.log, common: { noData: '暂无数据', refresh: '刷新' } },
    global: { directives: { permission: () => {}, tooltip: () => {} } },
  })
}

// 卸载时组件会清掉防抖 / 轮询定时器，避免上一个用例的定时器串到下一个用例
enableAutoUnmount(afterEach)

describe('KernelLogPanel', () => {
  beforeEach(() => {
    Object.values(api).forEach((f) => f.mockReset())
    api.getKernelLogOverview.mockResolvedValue({ data: overview() })
    api.listKernelLogEntries.mockResolvedValue({
      data: { file: CUR, entries, next_before: null, scanned_bytes: 10, file_size: 2048 },
    })
    api.getKernelLogStats.mockResolvedValue({
      data: {
        file: CUR,
        counts: { trace: 0, debug: 0, info: 1, warn: 0, error: 1, total: 2 },
        truncated: false,
      },
    })
  })

  it('渲染五级开关与计数；点击后 aria-pressed 切换', async () => {
    const w = mountPanel()
    await flushPromises()
    const levelBtns = w.findAll('.klog-level')
    expect(levelBtns).toHaveLength(6) // 全部 + 五级
    expect(levelBtns[0]!.attributes('aria-pressed')).toBe('true')
    const errBtn = w.find('.klog-level--error')
    expect(errBtn.text()).toContain('错误')
    expect(errBtn.find('.klog-level__count').text()).toBe('1')
    await errBtn.trigger('click')
    await flushPromises()
    expect(w.find('.klog-level--error').attributes('aria-pressed')).toBe('true')
    expect(w.find('.klog-level--all').attributes('aria-pressed')).toBe('false')
    expect(api.listKernelLogEntries).toHaveBeenLastCalledWith(
      expect.objectContaining({ levels: ['error'] }),
    )
  })

  it('行展开显示结构化字段与请求编号；「只看此请求」填入搜索', async () => {
    const w = mountPanel()
    await flushPromises()
    const rows = w.findAll('.klog-row')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.classes()).toContain('klog-row--error')
    expect(w.find('.klog-detail').exists()).toBe(false)

    await rows[0]!.find('.klog-row__main').trigger('click')
    expect(rows[0]!.find('.klog-row__main').attributes('aria-expanded')).toBe('true')
    const detail = w.find('.klog-detail')
    expect(detail.text()).toContain('req-abc')
    expect(detail.find('pre').text()).toContain('"error": "db locked"')

    const filterBtn = detail.findAll('button').find((b) => b.text().includes('只看此请求'))
    expect(filterBtn).toBeTruthy()
    await filterBtn!.trigger('click')
    expect((w.find('.klog-search input').element as HTMLInputElement).value).toBe('req-abc')
  })

  it('深链关键字：初始筛选并在消息中高亮命中片段', async () => {
    const w = mountPanel({ initialQuery: 'DB LOCKED' })
    await flushPromises()
    expect(api.listKernelLogEntries).toHaveBeenCalledWith(
      expect.objectContaining({ q: 'DB LOCKED' }),
    )
    const marks = w.findAll('.klog-row__msg mark')
    expect(marks).toHaveLength(1)
    expect(marks[0]!.text()).toBe('db locked')
  })

  it('当前文件的删除按钮置灰', async () => {
    const w = mountPanel()
    await flushPromises()
    const del = w.findAll('button').find((b) => b.find('.oi-trash').exists())
    expect(del).toBeTruthy()
    expect(del!.attributes('disabled')).toBeDefined()
  })

  it('未启用文件日志：展示配置指引，不渲染列表', async () => {
    api.getKernelLogOverview.mockResolvedValue({ data: overview(false) })
    const w = mountPanel()
    await flushPromises()
    expect(w.text()).toContain('OP_LOG_DIR')
    expect(w.find('.klog-list').exists()).toBe(false)
    expect(api.listKernelLogEntries).not.toHaveBeenCalled()
  })
})
