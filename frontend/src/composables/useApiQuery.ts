import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/vue-query'
import type { QueryKey } from '@tanstack/vue-query'
import { computed, isRef, ref } from 'vue'
import { onActivated, onDeactivated, onBeforeUnmount } from 'vue'
import type { Ref } from 'vue'
import { getErrorMessage } from '@/utils/error'

/**
 * F1.1 统一数据获取层封装。
 *
 * 统一了 useQuery / useMutation 的调用姿势，并提供 data / loading / error / refresh。
 *
 * 约定：
 * - 只读接口统一走 useQuery，queryKey 约定为模块名（+ 参数），同一 key 同屏共享缓存
 * - 写操作统一走 useMutation + invalidate 对应 key
 * - WS 推送优先 setQueryData 写缓存，其次 invalidate
 */

export interface UseApiQueryOptions {
  /** 默认 staleTime（默认 15s，走全局配置） */
  staleTime?: number
  /** 关闭自动重试（非幂等 GET 等场景） */
  retry?: boolean | number
  /** 是否自动执行（默认 true） */
  enabled?: boolean
  /** 分页场景保留上一页数据 */
  keepPrevious?: boolean
  /** 错误兜底文案（默认取 t('common.failed') 语义） */
  fallback?: string
  /** 轮询间隔（ms）。页签不可见时自动暂停（keep-alive 失活 / document 隐藏） */
  refetchInterval?: number | false | (() => number | false)
  /** 后台（tab 隐藏）是否继续轮询（默认 false） */
  refetchIntervalInBackground?: boolean
}

/**
 * useApiQuery — 基于 @tanstack/vue-query 的只读请求。
 * @param key 查询键（模块级前缀 + 参数）
 * @param fetcher 返回 AxiosResponse 形态（含 .data）的请求函数
 */
export function useApiQuery<T>(
  key: QueryKey | Ref<QueryKey> | (() => QueryKey),
  fetcher: () => Promise<{ data: T }>,
  options: UseApiQueryOptions = {},
) {
  const {
    staleTime,
    retry,
    enabled,
    keepPrevious = false,
    fallback = '操作失败',
    refetchInterval,
    refetchIntervalInBackground = false,
  } = options
  const queryKey = computed<QueryKey>(() => {
    if (typeof key === 'function') return key()
    if (isRef(key)) return key.value
    return key
  })

  // keep-alive 失活时暂停轮询（页签不可见即停止刷新）
  const keepAliveActive = ref(true)
  onActivated(() => {
    keepAliveActive.value = true
  })
  onDeactivated(() => {
    keepAliveActive.value = false
  })
  onBeforeUnmount(() => {
    keepAliveActive.value = false
  })

  const query = useQuery({
    queryKey,
    queryFn: async () => {
      const res = await fetcher()
      return res.data
    },
    staleTime,
    retry,
    enabled,
    placeholderData: keepPrevious ? keepPreviousData : undefined,
    refetchInterval:
      refetchInterval === undefined
        ? undefined
        : typeof refetchInterval === 'function'
          ? () => (keepAliveActive.value ? refetchInterval() : false)
          : () => (keepAliveActive.value ? (refetchInterval as number) : false),
    refetchIntervalInBackground,
  })

  const error = computed<string | null>(() => {
    if (!query.isError.value) return null
    const err = query.error.value
    return err ? getErrorMessage(err, fallback) : fallback
  })

  return {
    data: query.data as Ref<T | undefined>,
    loading: query.isLoading,
    isFetching: query.isFetching,
    isError: query.isError,
    error,
    refresh: query.refetch,
    query,
  }
}

export interface UseApiMutationOptions<TData, TVariables> {
  /** 成功后失效的 queryKey（可多个） */
  invalidates?: QueryKey[]
  /** 成功后通过 setQueryData 写入的 key + 更新函数（可选，用于 WS 双源合并） */
  onSuccess?: (data: TData, variables: TVariables) => void
  /** 失败回调（可选） */
  onError?: (err: unknown) => void
}

/**
 * useApiMutation — 基于 @tanstack/vue-query 的写操作。
 * 写完成后自动 invalidateQueries，保证「WS 一份、查询一份」最终一致。
 */
export function useApiMutation<TData, TVariables = void>(
  mutationFn: (variables: TVariables) => Promise<{ data: TData }>,
  options: UseApiMutationOptions<TData, TVariables> = {},
) {
  const queryClient = useQueryClient()
  const { invalidates = [], onSuccess, onError } = options

  const mutation = useMutation({
    mutationFn: async (variables: TVariables) => {
      const res = await mutationFn(variables)
      return res.data
    },
    onSuccess: (data, variables) => {
      onSuccess?.(data, variables)
      if (invalidates.length) {
        for (const key of invalidates) {
          void queryClient.invalidateQueries({ queryKey: key })
        }
      }
    },
    onError,
  })

  const error = computed<string | null>(() => {
    const err = mutation.error.value
    return err ? getErrorMessage(err, '操作失败') : null
  })

  return {
    ...mutation,
    loading: mutation.isPending,
    error,
  }
}

/** 供 WS 消息处理器使用：写缓存或失效 */
export function useQueryCacheClient() {
  return useQueryClient()
}

export { keepPreviousData }
