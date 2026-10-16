import api from './client'
import type { OverviewBadges } from '@/api/generated'

export type {
  OverviewBadges,
  NodeBadges,
  ContainerBadges,
  TaskBadges,
  AlertBadges,
} from '@/api/generated'

/**
 * S-06 顶栏概览聚合：节点 / 容器 / 任务 / 告警计数一次返回。
 *
 * 仅需登录；各分组按调用者角色权限裁剪，无权限或来源不可用时为 `null`
 * （顶栏据此隐藏对应徽标，不再各自请求列表接口在浏览器里数）。
 */
export function getOverviewBadges() {
  return api.get<OverviewBadges>('/overview/badges')
}
