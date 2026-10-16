import { computed, toValue, type MaybeRefOrGetter } from 'vue'
import { useApiMutation, useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import {
  killProcess,
  listProcesses,
  type KillProcessRequest,
  type KillProcessResponse,
  type ProcessListParams,
  type ProcessListResponse,
} from '@/api/toolbox'

/** 自动刷新间隔：服务端缓存 2 s，5 s 足够看清趋势且不给 /proc 施压 */
export const PROCESS_REFRESH_MS = 5_000

/** 去掉空值，保证 queryKey 稳定（`{ q: '' }` 与 `{}` 命中同一缓存） */
export function cleanParams(params: ProcessListParams): ProcessListParams {
  const out: ProcessListParams = {}
  const q = params.q?.trim()
  if (q) out.q = q
  if (params.user) out.user = params.user
  if (params.sort) out.sort = params.sort
  if (params.order) out.order = params.order
  if (params.limit) out.limit = params.limit
  if (params.include_kernel) out.include_kernel = true
  return out
}

/**
 * S-12 进程管理数据层：`GET /api/toolbox/processes` + `POST /api/toolbox/processes/{pid}/kill`。
 *
 * - 查询参数变化时 `keepPrevious` 保留上一份列表，搜索 / 排序切换不闪空；
 * - `autoRefresh` 为真时 5 s 轮询（keep-alive 失活 / 页签隐藏自动暂停）；
 * - 结束进程成功后失效整个 `toolbox` 前缀（服务端也已失效进程表缓存，下一次即新快照）。
 */
export function useProcesses(
  params: MaybeRefOrGetter<ProcessListParams>,
  options: { autoRefresh?: MaybeRefOrGetter<boolean> } = {},
) {
  const cleaned = computed(() => cleanParams(toValue(params)))

  const query = useApiQuery<ProcessListResponse>(
    () => queryKeys.toolbox.processes({ ...cleaned.value }),
    () => listProcesses(cleaned.value),
    {
      keepPrevious: true,
      staleTime: 2_000,
      retry: 1,
      refetchInterval: () => (toValue(options.autoRefresh) ? PROCESS_REFRESH_MS : false),
    },
  )

  const mutation = useApiMutation<KillProcessResponse, { pid: number; body: KillProcessRequest }>(
    ({ pid, body }) => killProcess(pid, body),
    { invalidates: [queryKeys.toolbox.all] },
  )

  const data = computed(() => query.data.value ?? null)

  return {
    data,
    rows: computed(() => data.value?.items ?? []),
    loading: query.loading,
    isFetching: query.isFetching,
    error: query.error,
    refresh: query.refresh,
    kill: (pid: number, body: KillProcessRequest) => mutation.mutateAsync({ pid, body }),
    killing: mutation.loading,
  }
}
