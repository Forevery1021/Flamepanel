import { computed } from 'vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getOverviewBadges } from '@/api/overview'
import type { OverviewBadges } from '@/api/overview'

/** 顶栏轮询间隔：30 s（服务端另有 10 s TTL 缓存，多页签只触发一次真实聚合） */
export const OVERVIEW_BADGES_INTERVAL = 30_000

/** 告警明细行（顺序即通知面板展示顺序） */
export interface AlertItem {
  /** i18n key 后缀：`topbar.alertNodesOffline` 等 */
  key: 'nodesOffline' | 'certsExpiring' | 'tasksFailed'
  count: number
  /** 点击跳转 */
  to: string
  icon: string
}

/**
 * S-06 顶栏概览徽标数据层。
 *
 * 替代 AppHeader 原先每 10 s 并发三个 REST（节点列表 200 条 / 容器列表 / 操作日志 6 条）
 * 并在浏览器里过滤计数的做法：现在只有 **一个** `GET /api/overview/badges`、30 s 一次、
 * 页签隐藏时不轮询（`refetchIntervalInBackground: false`）、切回前台按 staleTime 自动补刷。
 *
 * 分组为 `null` 有两种含义（服务端已合并，前端不区分）：调用者无对应 `*:read` 权限，
 * 或来源不可用（如 Docker 未安装）。两种情况顶栏都**隐藏**该项，而不是显示 0——
 * 显示 0 会让"没装 Docker"和"容器全停了"看起来一样。
 */
export function useOverviewBadges() {
  const query = useApiQuery<OverviewBadges>(queryKeys.overview.badges(), getOverviewBadges, {
    staleTime: 10_000,
    retry: 1,
    refetchInterval: OVERVIEW_BADGES_INTERVAL,
    refetchIntervalInBackground: false,
  })

  const badges = computed(() => query.data.value ?? null)

  /** 面板可达：拿到过数据且最近一次请求没失败 */
  const panelOk = computed(() => !query.isError.value)

  const nodes = computed(() => badges.value?.nodes ?? null)
  const containers = computed(() => badges.value?.containers ?? null)
  const tasks = computed(() => badges.value?.tasks ?? null)
  const alerts = computed(() => badges.value?.alerts ?? null)

  /** 有任务在跑（或排队）时顶栏显示进度指示 */
  const activeTasks = computed(() => (tasks.value ? tasks.value.running + tasks.value.pending : 0))

  const alertTotal = computed(() => alerts.value?.total ?? 0)

  /** 非零告警分项，供通知面板逐行展示 */
  const alertItems = computed<AlertItem[]>(() => {
    const a = alerts.value
    if (!a) return []
    const items: AlertItem[] = []
    if (a.nodes_offline) {
      items.push({ key: 'nodesOffline', count: a.nodes_offline, to: '/nodes', icon: 'oi-sitemap' })
    }
    if (a.certificates_expiring) {
      items.push({
        key: 'certsExpiring',
        count: a.certificates_expiring,
        to: '/websites',
        icon: 'oi-shield',
      })
    }
    if (a.tasks_failed) {
      items.push({ key: 'tasksFailed', count: a.tasks_failed, to: '/tasks', icon: 'oi-task' })
    }
    return items
  })

  return {
    badges,
    panelOk,
    nodes,
    containers,
    tasks,
    alerts,
    activeTasks,
    alertTotal,
    alertItems,
    loading: query.loading,
    refresh: query.refresh,
  }
}
