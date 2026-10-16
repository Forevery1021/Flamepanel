import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick, ref, type Ref } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import type { MetricsHistoryResponse } from '@/api/generated'
import type { TrendRange } from '@/utils/dashboardCharts'

/**
 * S-16 仪表盘时间范围数据层（`useMetricsHistory`）。
 *
 * 看住三件事：
 * ① `live` 不发请求（实时曲线仍由 WS 驱动，不能多出一路 REST 轮询）；
 * ② 切换范围 → 以新 range 请求，样本换成 S-07 历史点的均值映射（时间戳归一为 ms）；
 * ③ 切换期间 `keepPrevious`：新数据到达前保留上一范围的曲线，图表不闪空。
 */

const { api } = vi.hoisted(() => ({
  api: { getMetricsHistory: vi.fn() },
}))

vi.mock('@/api/metrics', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/metrics')>()
  return { ...actual, ...api }
})

import { useMetricsHistory, historyRefetchMs } from '@/composables/useMetricsHistory'

function response(range: string, stepSecs: number, buckets: number[]): MetricsHistoryResponse {
  return {
    range,
    step_secs: stepSecs,
    from: buckets[0] ?? 0,
    to: buckets[buckets.length - 1] ?? 0,
    points: buckets.map((bucket_start, i) => ({
      bucket_start,
      samples: 60,
      cpu_avg: 10 + i,
      cpu_max: 50,
      memory_avg: 20 + i,
      memory_max: 60,
      disk_avg: 30,
      load_one_avg: 0.5,
      net_rx_avg_mbps: 1,
      net_tx_avg_mbps: 2,
    })),
  } as MetricsHistoryResponse
}

const mounted: { unmount: () => void }[] = []

function mountHost(initial: TrendRange) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const range = ref<TrendRange>(initial)
  let ctl!: ReturnType<typeof useMetricsHistory>
  const Host = defineComponent({
    setup() {
      ctl = useMetricsHistory(range as Ref<TrendRange>)
      return {}
    },
    template: '<div/>',
  })
  const w = mount(Host, { global: { plugins: [[VueQueryPlugin, { queryClient }]] } })
  mounted.push(w)
  return { ctl, range }
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await nextTick()
    await new Promise((r) => setTimeout(r, 5))
  }
}

beforeEach(() => {
  api.getMetricsHistory.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('useMetricsHistory', () => {
  it('刷新间隔：分钟点每 60 s，小时点（7d / 30d）每 5 min', () => {
    expect(historyRefetchMs('1h')).toBe(60_000)
    expect(historyRefetchMs('24h')).toBe(60_000)
    expect(historyRefetchMs('7d')).toBe(5 * 60_000)
    expect(historyRefetchMs('30d')).toBe(5 * 60_000)
  })

  it('live 不请求历史：样本为空、无加载态', async () => {
    const { ctl } = mountHost('live')
    await settle()
    expect(api.getMetricsHistory).not.toHaveBeenCalled()
    expect(ctl.samples.value).toEqual([])
    expect(ctl.loading.value).toBe(false)
    expect(ctl.stepSecs.value).toBe(60)
  })

  it('切到 24h：以该范围请求，历史点映射为曲线样本（均值口径、秒 → ms）', async () => {
    api.getMetricsHistory.mockResolvedValue({
      data: response('24h', 60, [1_700_000_000, 1_700_000_060]),
    })
    const { ctl, range } = mountHost('live')
    await settle()

    range.value = '24h'
    await settle()

    expect(api.getMetricsHistory).toHaveBeenCalledTimes(1)
    expect(api.getMetricsHistory).toHaveBeenCalledWith('24h')
    expect(ctl.samples.value).toEqual([
      {
        timestamp: 1_700_000_000_000,
        cpu_usage: 10,
        memory_usage_percent: 20,
        disk_usage_percent: 30,
        network_rx_mbps: 1,
        network_tx_mbps: 2,
      },
      {
        timestamp: 1_700_000_060_000,
        cpu_usage: 11,
        memory_usage_percent: 21,
        disk_usage_percent: 30,
        network_rx_mbps: 1,
        network_tx_mbps: 2,
      },
    ])
    expect(ctl.stepSecs.value).toBe(60)
  })

  it('范围间切换保留上一范围样本直到新数据到达（keepPrevious，不闪空）', async () => {
    let release!: (v: { data: MetricsHistoryResponse }) => void
    api.getMetricsHistory
      .mockResolvedValueOnce({ data: response('1h', 60, [1_700_000_000]) })
      .mockImplementationOnce(() => new Promise((r) => (release = r)))
    const { ctl, range } = mountHost('1h')
    await settle()
    expect(ctl.samples.value).toHaveLength(1)

    range.value = '7d'
    await settle()
    // 7d 请求挂起中：仍显示 1h 的样本，且标记 isFetching
    expect(api.getMetricsHistory).toHaveBeenLastCalledWith('7d')
    expect(ctl.samples.value).toHaveLength(1)
    expect(ctl.isFetching.value).toBe(true)

    release({ data: response('7d', 3600, [1_699_996_400, 1_700_000_000, 1_700_003_600]) })
    await settle()
    expect(ctl.samples.value).toHaveLength(3)
    expect(ctl.stepSecs.value).toBe(3600)
    expect(ctl.isFetching.value).toBe(false)
  })

  it('请求失败：重试一次后暴露 error；切回 live 后停止请求', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    try {
      api.getMetricsHistory.mockRejectedValue(new Error('boom'))
      const { ctl, range } = mountHost('1h')
      await settle()
      // 首次失败 → 1 s 后重试一次 → 才进入错误态
      expect(ctl.error.value).toBeNull()
      await vi.advanceTimersByTimeAsync(1100)
      await settle()
      expect(api.getMetricsHistory).toHaveBeenCalledTimes(2)
      expect(ctl.error.value).toBeTruthy()
      expect(ctl.samples.value).toEqual([])

      range.value = 'live'
      await settle()
      await vi.advanceTimersByTimeAsync(5000)
      expect(api.getMetricsHistory).toHaveBeenCalledTimes(2)
    } finally {
      vi.useRealTimers()
    }
  })
})
