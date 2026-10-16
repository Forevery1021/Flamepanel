/**
 * 仪表盘展示纯函数（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 这些映射原先散落在 851 行的 DashboardView 内联实现里，被指标卡网格
 * （四张卡的 `value` / `percent` / `detail` / 跳转目标）、趋势图与负载仪表盘
 * （三处共用 `chartPalette` 的取色）、网络面板（MB/s 数值）多处共用。
 * 抽成纯函数后既可单测，也避免拆分子组件时「复制多份、改一处漏两处」——
 * 原文件里 `usageColor` 与四张卡的拼装都只写了一份，拆分时若各组件各抄一份
 * 就正好会踩这个坑。
 *
 * 纯函数边界：本文件不 import vue / echarts，也不读写 DOM——取色、拼装、
 * 格式化一律入参出参，便于单测锁死阈值与四舍五入口径。
 */

import type { MetricsSnapshot } from '@/types'

/** 指标卡键（与 `stat-<key>` 配色类一一对应） */
export type StatCardKey = 'cpu' | 'mem' | 'disk' | 'load'

/** 指标卡模型（视图层只负责渲染，不再做题面计算） */
export interface DashboardStatCard {
  key: StatCardKey
  icon: string
  label: string
  value: string
  percent: number
  detail: string
  /** 主数值语义色令牌；原实现遗留字段（模板未消费），保留以维持行为逐行对齐 */
  color: string
  /** 点击跳转目标（无则不渲染箭头、不可点击） */
  to: string
}

/** 负载仪表盘量程上限（gauge 的 max 与卡片进度条映射同源，避免两处各写 10） */
export const LOAD_GAUGE_MAX = 10

/**
 * 使用率 → 语义色令牌：>80 红 / >50 黄 / 其余绿。
 *
 * 阈值是「运维一眼知健康」的判据，边界取值刻意保持原实现的 `>`（80 仍为黄、
 * 50 仍为绿）——改成 `>=` 会让恰好 80% 的机器变红，属行为变更。
 */
export function usageColor(percent: number): string {
  return percent > 80 ? 'var(--fp-danger)' : percent > 50 ? 'var(--fp-warning)' : 'var(--fp-success)'
}

/** MB/s 数值展示：缺失回落 0（原实现的 `?? 0` 由更新链路保证，此处兜底） */
export function formatMbps(value: number | null | undefined): string {
  return (value ?? 0).toFixed(2)
}

/**
 * 四张指标卡的题面拼装（CPU / 内存 / 磁盘 / 负载）。
 *
 * 三处易错口径均有用例锁死：
 * - 负载卡进度条按 `load_one / 10` 映射并**封顶 100**（负载可超过 10，
 *   不封顶会出现超出轨道的进度条）；
 * - 内存/磁盘的 detail 走 MB→GB、GB 直读的两种单位换算，各自 `toFixed(1)`；
 * - 跳转目标与卡片一一对应（CPU/内存 → 节点、磁盘 → 文件、负载 → 健康）。
 */
export function buildStatCards(
  snap: MetricsSnapshot,
  t: (key: string) => string,
): DashboardStatCard[] {
  return [
    {
      key: 'cpu',
      icon: 'oi-microchip',
      label: t('dashboard.cpu'),
      value: `${snap.cpu_usage.toFixed(1)}%`,
      percent: Math.round(snap.cpu_usage),
      detail: `${snap.cpu_cores} cores`,
      color: usageColor(snap.cpu_usage),
      to: '/nodes',
    },
    {
      key: 'mem',
      icon: 'oi-server',
      label: t('dashboard.memory'),
      value: `${snap.memory_usage_percent.toFixed(1)}%`,
      percent: Math.round(snap.memory_usage_percent),
      detail: `${(snap.memory_used_mb / 1024).toFixed(1)} / ${(snap.memory_total_mb / 1024).toFixed(1)} GB`,
      color: usageColor(snap.memory_usage_percent),
      to: '/nodes',
    },
    {
      key: 'disk',
      icon: 'oi-database',
      label: t('dashboard.disk'),
      value: `${snap.disk_usage_percent.toFixed(1)}%`,
      percent: Math.round(snap.disk_usage_percent),
      detail: `${snap.disk_used_gb.toFixed(1)} / ${snap.disk_total_gb.toFixed(1)} GB`,
      color: usageColor(snap.disk_usage_percent),
      to: '/files',
    },
    {
      key: 'load',
      icon: 'oi-wave-pulse',
      label: t('dashboard.load'),
      value: snap.load_one.toFixed(2),
      percent: Math.min(Math.round((snap.load_one / LOAD_GAUGE_MAX) * 100), 100),
      detail: `1m: ${snap.load_one.toFixed(2)} | 5m: ${snap.load_five.toFixed(2)} | 15m: ${snap.load_fifteen.toFixed(2)}`,
      color: 'var(--fp-success)',
      to: '/health',
    },
  ]
}

/** 指标卡三段色阶：<60 正常 / <85 注意 / ≥85 危险；负载卡（0–10 量程映射的百分比）阈值 50/80 */
export type StatTone = 'ok' | 'warn' | 'bad'
export function statTone(percent: number, key: string): StatTone {
  if (key === 'load') return percent >= 80 ? 'bad' : percent >= 50 ? 'warn' : 'ok'
  return percent >= 85 ? 'bad' : percent >= 60 ? 'warn' : 'ok'
}

/**
 * 迷你趋势线 path（viewBox 100×28，上下各留 2px）。
 * 单趟求极值 + 字符串拼接，无中间数组；少于 2 点返回空串（调用方据此不渲染 svg）。
 */
export function sparkPath(v: readonly number[]): string {
  if (v.length < 2) return ''
  let min = Infinity
  let max = -Infinity
  for (const n of v) {
    if (n < min) min = n
    if (n > max) max = n
  }
  const span = max - min || 1
  const step = 100 / (v.length - 1)
  let d = ''
  for (let i = 0; i < v.length; i++) {
    d += `${i === 0 ? 'M' : ' L'}${(i * step).toFixed(1)},${(26 - ((v[i] - min) / span) * 24).toFixed(1)}`
  }
  return d
}
