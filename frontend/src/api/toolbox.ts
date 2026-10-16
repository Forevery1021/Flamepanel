import api from './client'
import type {
  HostDetails,
  KillProcessRequest,
  KillProcessResponse,
  ProcessListResponse,
} from '@/api/generated'

export type {
  DnsConfig,
  HostCpu,
  HostDetails,
  HostDisk,
  HostInterface,
  HostMemory,
  HostsEntry,
  HostSystem,
  HostTime,
  PanelRuntime,
  KillProcessRequest,
  KillProcessResponse,
  ProcessInfo,
  ProcessListResponse,
  ProcessRow,
  ProcessSignal,
  ProtectedReason,
} from '@/api/generated'

/** 进程列表排序字段（与后端 `sort` 参数一致） */
export const PROCESS_SORTS = ['cpu', 'memory', 'pid', 'name', 'started'] as const
export type ProcessSortField = (typeof PROCESS_SORTS)[number]

/** 允许发送的信号（后端只接受这 4 个） */
export const PROCESS_SIGNALS = ['TERM', 'KILL', 'HUP', 'INT'] as const

export interface ProcessListParams {
  q?: string
  user?: string
  sort?: ProcessSortField
  order?: 'asc' | 'desc'
  limit?: number
  include_kernel?: boolean
}

/**
 * S-12 进程列表（需 `toolbox:read`）。服务端 2 s TTL + 单飞：多标签页并发轮询只扫一次 `/proc`；
 * 命令行已在服务端脱敏（`--password=***`）。
 */
export function listProcesses(params: ProcessListParams = {}) {
  return api.get<ProcessListResponse>('/toolbox/processes', { params })
}

/**
 * 向进程发送信号（需 `toolbox:execute`）。带上列表里的 `started_at`：
 * 进程已重启 / pid 被复用时服务端返回 409，而不是误杀新进程。
 */
export function killProcess(pid: number, body: KillProcessRequest = {}) {
  return api.post<KillProcessResponse>(`/toolbox/processes/${pid}/kill`, body)
}

/**
 * 系统工具箱 · 主机详情（需 `toolbox:read`）：系统 / CPU / 内存 / 时区 / DNS / hosts / 网卡 / 磁盘 /
 * 面板自身。只读，任何事实不可探测时对应字段为空而不是报错。
 */
export function getHostDetails() {
  return api.get<HostDetails>('/toolbox/host')
}
