import { computed } from 'vue'
import { useApiMutation, useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getInspectionReport, runInspection, type InspectionReport } from '@/api/inspection'
import { groupFindings, urgentFindings } from '@/utils/inspection'

/** 页面驻留时的后台补刷间隔：与服务端 10 min 缓存同步，一次刷新最多触发一次真实采集 */
export const INSPECTION_REFETCH_MS = 10 * 60_000

/**
 * S-17 系统体检数据层：`GET /api/inspection`（缓存报告）+ `POST /api/inspection/run`（强制重跑）。
 *
 * - 报告在页面与仪表盘横幅之间共享同一 queryKey：两处同时挂载只打一次请求；
 * - `run()` 成功后直接把新报告写入缓存（不再多一次 GET），失败交给 `runError`；
 * - `enabled = false` 用于无 `toolbox:read` 的角色（仪表盘横幅按权限决定是否订阅）。
 */
export function useInspection(options: { enabled?: boolean } = {}) {
  const queryClient = useQueryCacheClient()

  const query = useApiQuery<InspectionReport>(queryKeys.inspection.report(), getInspectionReport, {
    enabled: options.enabled ?? true,
    staleTime: 60_000,
    // 首次采集可能需要 1–2 s；失败只重试一次，尽快呈现错误态
    retry: 1,
    refetchInterval: INSPECTION_REFETCH_MS,
    refetchIntervalInBackground: false,
  })

  const mutation = useApiMutation<InspectionReport, void>(() => runInspection(), {
    onSuccess: (report) => {
      queryClient.setQueryData(queryKeys.inspection.report(), report)
    },
  })

  const report = computed(() => query.data.value ?? null)
  const groups = computed(() => (report.value ? groupFindings(report.value.findings) : []))
  const urgent = computed(() => (report.value ? urgentFindings(report.value) : []))

  return {
    report,
    groups,
    urgent,
    loading: query.loading,
    isFetching: query.isFetching,
    error: query.error,
    refresh: query.refresh,
    run: () => mutation.mutateAsync(),
    running: mutation.loading,
    runError: mutation.error,
  }
}
