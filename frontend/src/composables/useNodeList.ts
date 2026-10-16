import { computed, onMounted, onUnmounted, watch } from 'vue'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { useCrudPage } from '@/composables/useCrudPage'
import { queryKeys } from '@/api/queryKeys'
import { listNodes, nodeStatus, nodeMetrics } from '@/api/nodes'
import { enrichNode, type EnrichedNode } from '@/utils/nodeFormat'
import type { NodeMetrics, ServerNode } from '@/types'

/**
 * 节点列表数据层（Doc/23 §3.3 T2-4 视图拆分第七站）。
 *
 * 原 NodesView（824 行）把「列表加载 + 分页 + 在线状态/指标轮询写缓存 + 增删改 +
 * 远程命令 + 批量命令 + 远程文件」全部塞在同一文件。本 composable 收敛列表族：
 *
 * - 列表查询（`useApiQuery`，`queryKeys.nodes.list`，**10s 轮询**、后台不轮询）；
 * - 分页（`useCrudPage`，`keepPrevious` 保留上一页）；
 * - **在线状态与实时指标的二次覆盖**（本站数据流的独特之处，见下）；
 * - 页面从后台切回时的即时补刷（轮询间隔 10s，切回来若干等最多 10s 才更新）。
 *
 * ## 为什么在线状态要「二次覆盖」而不能直接在富化时算
 *
 * 列表接口返回的 `status` 是**注册时填写的期望值**，Agent 是否真的在线只能由
 * `/status`（心跳判定）与 `/metrics` 给出。故数据流是两段：
 *
 * 1. 列表响应 → `enrichNode`（解析 `metrics_json`，`online` 先置 false）；
 * 2. 列表加载完成（含首屏 / 翻页 / 轮询）→ 逐节点并发查 `/status` + `/metrics`，
 *    **写回查询缓存**（`setQueryData`）而不是另存一份 ref。
 *
 * 写回缓存是本层最关键的一条：视图里只有「查询缓存」这一个数据源，WS / 轮询 /
 * 手动刷新谁先到都不会出现两份状态互相打架（原实现的注释即「WS 列表双源合一」）。
 *
 * 状态查询失败**保留上一次的指标**（只把 online 置 false）：Agent 刚掉线时
 * 最后一次上报的 CPU/内存仍有诊断价值，抹成「—」反而丢失了「掉线前的样子」。
 */
export function useNodeList() {
  const queryClient = useQueryCacheClient()

  const crud = useCrudPage({ pageSize: 20 })
  const { total, first, pageSize, onFirst } = crud

  /** 列表缓存形状：`data` 为富化行、`total` 为后端总条数（原实现的形态，保持不变） */
  interface NodeListCache {
    data: EnrichedNode[]
    total: number
  }

  const nodesQuery = useApiQuery<NodeListCache>(
    () => queryKeys.nodes.list(crud.currentPage.value, crud.pageSize.value),
    async () => {
      const res = await listNodes(crud.currentPage.value, crud.pageSize.value)
      const enriched = res.data.data.map(enrichNode)
      crud.total.value = res.data.total
      return { data: { data: enriched, total: res.data.total } }
    },
    { keepPrevious: true, refetchInterval: 10_000, refetchIntervalInBackground: false },
  )

  const nodes = computed<EnrichedNode[]>(() => nodesQuery.data.value?.data ?? [])
  const loading = nodesQuery.loading

  function fetch() {
    // 仅刷新列表；在线状态/指标由 isFetching 回落时自动覆盖，避免重复请求
    return nodesQuery.refresh()
  }

  /**
   * 逐节点查在线状态 + 指标（并发），结果写回查询缓存。
   *
   * 任一节点的查询失败**不阻断其余节点**（`Promise.all` 里各自 catch）——
   * 一台 Agent 不可达不该让整张表的指标停在旧值上。
   */
  async function refreshStatus() {
    if (!nodes.value.length) return
    const updated = await Promise.all(
      nodes.value.map(async (node) => {
        try {
          const [st, mt] = await Promise.all([nodeStatus(node.id), nodeMetrics(node.id)])
          return {
            ...node,
            online: st.data.status === 'online',
            metrics: mt.data as NodeMetrics,
            edge: st.data.edge ?? null,
          }
        } catch {
          return { ...node, online: false, edge: null }
        }
      }),
    )
    queryClient.setQueryData(
      queryKeys.nodes.list(crud.currentPage.value, crud.pageSize.value),
      (old: NodeListCache | undefined) => (old ? { data: updated, total: old.total } : old),
    )
  }

  /** 页面从后台切回：立即补刷一次状态（不干等下一个轮询点） */
  let lastVisible = document.visibilityState === 'visible'

  function onVisibility() {
    const visible = document.visibilityState === 'visible'
    if (visible && !lastVisible) void refreshStatus()
    lastVisible = visible
  }

  // 列表每次加载完成（含首屏 / 翻页 / 轮询）后覆盖在线状态与指标
  watch(
    nodesQuery.isFetching,
    (fetching) => {
      if (!fetching) void refreshStatus()
    },
    { flush: 'post' },
  )

  onMounted(() => document.addEventListener('visibilitychange', onVisibility))
  onUnmounted(() => document.removeEventListener('visibilitychange', onVisibility))

  return {
    nodes,
    loading,
    total,
    first,
    pageSize,
    onFirst,
    fetch,
    refreshStatus,
  }
}

/** 列表行类型（展示层与富化产物共用同一形状） */
export type { EnrichedNode, ServerNode }
