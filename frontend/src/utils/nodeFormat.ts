/**
 * 节点域的展示映射与载荷组装纯函数（Doc/23 §3.3 T2-4 视图拆分第七站）。
 *
 * 这些映射原先散落在 824 行的 NodesView 内联实现里，被表格（状态列 / 指标条 /
 * 心跳列）、注册与编辑弹窗（同一套字段校验写了两遍）、远程文件弹窗（路径拼接、
 * 上跳、base64 编解码）三处共用。抽成纯函数后既可单测，也避免拆分子组件时
 * 「复制多份、改一处漏两处」——原文件里 `percentColor` 与 `validateForm` /
 * `validateEditForm` 就是两份逐行相同的实现。
 *
 * 纯函数边界：本文件不 import vue / axios，也不读写 DOM——解析、取色、拼接、
 * 编解码一律入参出参，便于单测锁死阈值与四舍五入口径。
 */

import { usageColor } from './dashboardFormat'
import type { NodeEdgeInfo, NodeMetrics, ServerNode } from '@/types'

/**
 * 使用率 → 语义色令牌：>80 红 / >50 黄 / 其余绿。
 *
 * **与仪表盘 `usageColor` 同源（直接复用，不抄第二份）**：阈值是「运维一眼知
 * 健康」的判据，两处各写一份必然会在某次调阈值时只改一处——节点页把 80% 的机器
 * 显成黄、仪表盘显成红，是最难向用户解释的那类不一致。原 NodesView 里的
 * `percentColor` 与 `usageColor` 就是同一份逻辑的两个拷贝。
 */
export const percentColor = usageColor

/** 节点 + 实时指标/在线状态（列表主查询的富化产物） */
export interface EnrichedNode extends ServerNode {
  metrics: NodeMetrics | null
  online: boolean
  /** Agent Edge 通道（`/status` 覆盖后才有；null = 未知 / 未连接） */
  edge?: NodeEdgeInfo | null
}

/**
 * `metrics_json` 文本 → 指标对象。
 *
 * 解析失败回落 `null` 而不是抛错：该字段是 Agent 上报的历史快照，脏数据
 * （半截写入、版本错配）不该让整个节点列表打不开——回落成「—」即可
 * （原实现的 `try/catch` 口径，拆分时原样保留）。
 */
export function parseNodeMetrics(metricsJson: string | null | undefined): NodeMetrics | null {
  if (!metricsJson) return null
  try {
    return JSON.parse(metricsJson) as NodeMetrics
  } catch {
    return null
  }
}

/**
 * 列表行 → 富化行：解析指标快照，在线状态先置 `false`。
 *
 * **在线状态不参与富化**（这是本站最容易写错的一处）：`status` 字段是注册时
 * 填的期望值，真实在线与否只能由 `/status` 与心跳时刻判定——若把 `status ===
 * 'online'` 当作在线，离线的机器会显示绿点，直到下一次状态轮询才纠正
 * （原实现即「先置 false，再由状态层覆盖」，拆分时保留）。
 */
export function enrichNode(node: ServerNode): EnrichedNode {
  return { ...node, metrics: parseNodeMetrics(node.metrics_json), online: false }
}

/**
 * 指标条样式：**宽度取四舍五入的整数百分比**（原实现即如此）。
 *
 * 刻意不夹取到 100：CPU 读数超过 100% 是容器/采样窗口的正常现象
 * （多核归一差异），夹取会把「读数异常」伪装成「满格」——运维看到满格就
 * 不会再深究，而看到 137% 会去查为什么。
 */
export function cpuBarStyle(metrics: NodeMetrics | null): { width: string; background: string } {
  const cpu = metrics?.cpu_usage ?? 0
  return { width: `${Math.round(cpu)}%`, background: percentColor(cpu) }
}

/**
 * 指标摘要文本：`CPU 12.3% · MEM 45.6% · DISK 78.9%`。
 *
 * 三个值一律 `toFixed(1)`（原实现口径）：整数百分比看不出 0.5% 的抖动，
 * 而运维判断「这台机器是不是刚起了一个进程」靠的正是小数位。
 * 缺值回落 0（快照字段可能只上报了部分维度）。
 */
export function nodeMetricsSummary(metrics: NodeMetrics | null): string {
  const cpu = (metrics?.cpu_usage ?? 0).toFixed(1)
  const mem = (metrics?.memory_usage_percent ?? 0).toFixed(1)
  const disk = (metrics?.disk_usage_percent ?? 0).toFixed(1)
  return `CPU ${cpu}% · MEM ${mem}% · DISK ${disk}%`
}

/** 节点状态下拉候选项（online / offline 两档，译文按当前语言现算） */
export function nodeStatusOptions(t: (key: string) => string): { label: string; value: string }[] {
  return [
    { label: t('dashboard.online'), value: 'online' },
    { label: t('dashboard.offline'), value: 'offline' },
  ]
}

/** 远程路径拼接：根目录 `.` 下直接落名字，否则用 `/` 连接 */
export function joinRemotePath(parent: string, name: string): string {
  return parent === '.' ? name : `${parent}/${name}`
}

/**
 * 远程路径上跳一级（`.` 之上仍是 `.`）。
 *
 * 先按 `/` 切分再 `filter(Boolean)`：这样 `/a/b`（前导斜杠）与 `a/b` 得到同一
 * 结果、`a//b` 也不会把空段当成一层（原实现口径）。
 */
export function parentRemotePath(path: string): string {
  const parts = path.split('/').filter(Boolean)
  parts.pop()
  return parts.length ? parts.join('/') : '.'
}

/** UTF-8 文本 → base64（远程上传载荷） */
export function encodeBase64(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let binary = ''
  for (const b of bytes) binary += String.fromCharCode(b)
  return btoa(binary)
}

/**
 * base64 → 字节（远程下载落盘）。
 *
 * 不用 `TextDecoder`：下载的是**文件内容**（可能是 tar / 图片），任何按文本
 * 解码的中间步骤都会改字节；这里只做 base64 → 二进制。
 */
export function decodeBase64(contentBase64: string): Uint8Array {
  return Uint8Array.from(atob(contentBase64), (c) => c.charCodeAt(0))
}

/** 节点表单态（注册与编辑共用同一组字段） */
export interface NodeFormState {
  name: string
  hostname: string
  ip_address: string
  status: string
}

/** 字段级错误（空串 = 该字段通过） */
export interface NodeFieldErrors {
  name: string
  hostname: string
  ip_address: string
}

/** 注册弹窗初值（status 默认 online——新建节点的期望态） */
export function initNodeForm(): NodeFormState {
  return { name: '', hostname: '', ip_address: '', status: 'online' }
}

/**
 * 行数据 → 编辑表单回填。
 *
 * 编辑态刻意**不带 `metrics_json` / `last_heartbeat_at`**：它们不是可编辑字段，
 * 带回表单只会在提交时被当成用户输入送回后端（原实现即只取四个字段）。
 */
export function nodeFormFrom(node: ServerNode): NodeFormState {
  return {
    name: node.name,
    hostname: node.hostname,
    ip_address: node.ip_address,
    status: node.status,
  }
}

/**
 * 字段校验（注册与编辑共用；原文件里两份逐行相同的函数在此收敛为一份）。
 *
 * 三项必填均按「非空字符串」判定，**不做格式校验**：`hostname` 允许短名、
 * `ip_address` 允许域名（Agent 也走主机名解析），格式收紧会挡住真实可用的
 * 内网短名——这类拦截应发生在 Agent 连通测试那一刻，而不是输完就红。
 */
export function validateNodeFields(
  form: NodeFormState,
  t: (key: string) => string,
): NodeFieldErrors {
  return {
    name: form.name ? '' : t('node.nameRequired'),
    hostname: form.hostname ? '' : t('node.hostnameRequired'),
    ip_address: form.ip_address ? '' : t('node.ipRequired'),
  }
}

/** 字段错误是否全部通过 */
export function hasFieldErrors(errors: NodeFieldErrors): boolean {
  return Boolean(errors.name || errors.hostname || errors.ip_address)
}

/**
 * 更新请求体：`created_at` 送空串。
 *
 * 后端更新接口收的是整份 `ServerNode`，而列表行里带的是**展示用**的快照
 * （含 `metrics_json` / 心跳时刻等 Agent 上报字段）。把它们原样回送会让面板
 * 手里那份过期快照覆盖掉 Agent 刚写的新值，故只送四个可编辑字段 +
 * 后端要求的 `created_at` 占位（原实现口径）。
 */
export function buildNodeUpdateBody(id: number, form: NodeFormState): ServerNode {
  return {
    id,
    name: form.name,
    hostname: form.hostname,
    ip_address: form.ip_address,
    status: form.status,
    created_at: '',
  }
}
