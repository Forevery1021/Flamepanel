import { useQueryClient } from '@tanstack/vue-query'
import { QueryClient, QueryCache, MutationCache, VueQueryPlugin } from '@tanstack/vue-query'
import { reportError } from '@/utils/monitor'

export { VueQueryPlugin }
export type { QueryClient }

/**
 * QueryClient 全局配置（F1.1 统一数据获取层）。
 * - staleTime: 只读高频接口默认 15s 内复用缓存，避免同屏多组件重复打请求
 * - gcTime: 离开页面 5 分钟后清理
 * - retry: 与后端速率限制匹配（幂等 GET 重试 2 次，非幂等写操作在 mutation 中配置）
 * - refetchOnWindowFocus: 窗口聚焦恢复时静默刷新，保持数据新鲜
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => {
        // 全局查询错误兜底：结构化上报（视图层已通过 useApiQuery 展示 UI 提示，这里仅记录）
        reportError(error, { source: 'query', context: { queryKey: query.queryKey } })
      },
    }),
    mutationCache: new MutationCache({
      onError: (error) => {
        // 全局写操作错误兜底：结构化上报
        reportError(error, { source: 'mutation' })
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: 15_000,
        gcTime: 5 * 60_000,
        retry: 2,
        refetchOnWindowFocus: true,
        refetchOnReconnect: true,
      },
      mutations: {
        retry: 0,
      },
    },
  })
}

/**
 * 获取全局 queryClient（需在 app.use(VueQueryPlugin) 之后调用）。
 * 供 WS 消息处理器使用：setQueryData 写缓存 / invalidateQueries 失效。
 */
export function useQueryCache() {
  return useQueryClient()
}
