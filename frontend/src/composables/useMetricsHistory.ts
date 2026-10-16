import { computed, type Ref } from 'vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { getMetricsHistory, type MetricsHistoryRange } from '@/api/metrics'
import type { MetricsHistoryResponse } from '@/api/generated'
import { queryKeys } from '@/api/queryKeys'
import { historyToTrendSamples, type TrendRange, type TrendSample } from '@/utils/dashboardCharts'

/** 各范围的后台刷新间隔：1 min 点每分钟补一格，1 h 点 5 min 一次足够 */
export function historyRefetchMs(range: MetricsHistoryRange): number {
  return range === '7d' || range === '30d' ? 5 * 60_000 : 60_000
}

/**
 * S-16 仪表盘时间范围：消费 S-07 `GET /api/metrics/history?range=`。
 *
 * - `range === 'live'` 时不请求（实时 6 min 仍由 `/ws/metrics` 驱动）；
 * - 切换范围时 `keepPrevious` 保留上一范围曲线直到新数据到达，避免图表闪空；
 * - 轮询走 `useApiQuery`（keep-alive 失活 / 页签隐藏自动暂停）。
 */
export function useMetricsHistory(range: Ref<TrendRange>) {
  const enabled = computed(() => range.value !== 'live')
  const apiRange = computed<MetricsHistoryRange>(() =>
    range.value === 'live' ? '1h' : range.value,
  )

  const query = useApiQuery<MetricsHistoryResponse>(
    () => queryKeys.metrics.history(apiRange.value),
    () => getMetricsHistory(apiRange.value),
    {
      enabled,
      keepPrevious: true,
      // 历史曲线不是关键路径：失败只重试一次，尽快把错误态交给面板徽标
      retry: 1,
      staleTime: 30_000,
      refetchInterval: () => historyRefetchMs(apiRange.value),
    },
  )

  const samples = computed<TrendSample[]>(() =>
    query.data.value ? historyToTrendSamples(query.data.value.points) : [],
  )
  const stepSecs = computed(() => query.data.value?.step_secs ?? 60)

  return {
    samples,
    stepSecs,
    loading: query.loading,
    isFetching: query.isFetching,
    error: query.error,
    refresh: query.refresh,
  }
}
