/**
 * Docker 展示与过滤的公共纯函数（Doc/23 §3.3 T2-4 视图拆分的公共纯函数层）。
 *
 * 这些映射原先散落在 1107 行的 DockerView 内联实现里，被容器表（状态标签）、
 * 日志弹窗（行数候选）、网络与卷弹窗（驱动候选）、连接弹窗（容器候选）、
 * 四类清理确认（目标名）多处共用。抽成纯函数后既可单测，也避免拆分子组件时
 * 「复制多份、改一处漏两处」——原文件里 `statusType` 与四处下拉候选都只写了一份，
 * 但拆分时若各组件各抄一份就正好会踩这个坑。
 */

import type { DockerContainer } from '@/types'

/** FpTag 支持的语义色（与 FpTag 的 severity 取值同源，避免各调用点各写一份字面量） */
export type ContainerStatusType = 'success' | 'warning' | 'danger' | 'info'

/**
 * 容器状态 → 标签配色：running 绿 / paused 黄 / exited·dead 红 / 其余中性。
 *
 * 未知状态**不猜**（回落 info 而非默认绿）：Docker 会返回 created、restarting、
 * removing 等值，把未知态显示成「运行中」是最容易误导运维的映射错误。
 */
export function containerStatusType(status: string): ContainerStatusType {
  if (status === 'running') return 'success'
  if (status === 'paused') return 'warning'
  if (status === 'exited' || status === 'dead') return 'danger'
  return 'info'
}

/** 容器搜索：名称 / ID / 镜像三字段包含匹配（大小写不敏感；空查询返回原数组） */
export function filterContainers(rows: DockerContainer[], query: string): DockerContainer[] {
  const q = query.trim().toLowerCase()
  if (!q) return rows
  return rows.filter(
    (c) =>
      c.name.toLowerCase().includes(q) ||
      c.id.toLowerCase().includes(q) ||
      c.image.toLowerCase().includes(q),
  )
}

/**
 * 日志行数候选：前三档加「全部」。
 *
 * 「全部」沿用原实现的 9999 行哨兵值（后端按 tail 参数截断），末项标签不带数字——
 * 它表达的是「尽量多」，给一个「无限」的假象反而不准确。
 */
export function logTailOptions(t: (key: string) => string) {
  return [
    { label: `50 ${t('docker.tailLines')}`, value: 50 },
    { label: `100 ${t('docker.tailLines')}`, value: 100 },
    { label: `500 ${t('docker.tailLines')}`, value: 500 },
    { label: t('docker.tailLines'), value: 9999 },
  ]
}

/** 网络驱动白名单（与后端 createNetwork 接受的值一致） */
export const NETWORK_DRIVERS = ['bridge', 'host', 'overlay', 'macvlan'] as const

/** 卷驱动白名单 */
export const VOLUME_DRIVERS = ['local', 'nfs'] as const

/** 驱动候选：驱动名是 Docker 标识符，不做翻译（翻译反而对不上官方文档） */
export function driverOptions(drivers: readonly string[]): { label: string; value: string }[] {
  return drivers.map((d) => ({ label: d, value: d }))
}

/**
 * 连接容器的下拉候选：以容器名为标签、ID 为值。
 *
 * 名称为空时回落 ID——容器刚创建而名字未就绪时（或改名竞态）下拉不出现空标签。
 */
export function connectContainerOptions(
  rows: DockerContainer[],
): { label: string; value: string }[] {
  return rows.map((c) => ({ label: c.name || c.id, value: c.id }))
}
