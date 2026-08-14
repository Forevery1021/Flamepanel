/**
 * 前端业务类型（F4.1 类型收紧）。
 *
 * 规则：
 * - 后端 OpenAPI 已覆盖的实体类型（User / ServerNode / Website / ScheduledTask /
 *   SettingEntry / HealthDetail / ProcessEntry / LoginResponse / AppMetadata /
 *   InstalledApp / BackupEntry ...）统一从 `@/api/generated` 导入，禁止在本文件重复定义
 *   （避免与后端字段漂移）。见 `src/api/generated/index.ts`。
 * - 本文件只保留后端 OpenAPI 未覆盖的端点类型（Docker / Files / Firewall / Memos /
 *   Plugins / Databases / WebServers / Metrics / OperationLog / Log ...）。
 */
import type { User as GenUser } from '@/api/generated'

/* ── 后端 OpenAPI 已覆盖实体（由 generated 导出，此处仅复导出） ── */
export type {
  User,
  ServerNode,
  Website,
  ScheduledTask,
  SettingEntry,
  HealthDetail,
  ProcessEntry,
  LoginRequest,
  LoginResponse,
  AppMetadata,
  AppStoreListResponse,
  InstalledApp,
  InstalledAppResponse,
  BackupEntry,
  Page as PaginatedResponse,
  CreateUserRequest,
  UpdateUserRequest,
  CreateNodeRequest,
  CreateWebsiteRequest,
  UpdateSettingRequest,
  CreateTaskRequest,
  UpdateTaskRequest,
  RemoteExecRequest,
  RemoteBatchExecRequest,
  RemoteUploadRequest,
  HeartbeatRequest,
  Schema,
} from '@/api/generated'

/** 兼容旧引用：User 从 generated 重导出（含 must_change_password） */
export type GenUserAlias = GenUser

/* ═══════════ 后端 OpenAPI 未覆盖的端点类型 ═══════════ */

export interface RemoteFileEntry {
  name: string
  is_dir: boolean
  size: number
  modified: string
}

export interface RemoteExecResult {
  node_id?: number
  output: string
  exit_code: number
  duration_ms: number
}

export interface BatchExecItem {
  node_id: number
  node_name: string
  success: boolean
  result: { output?: string; exit_code?: number; duration_ms?: number; error?: string }
}

export interface NodeMetrics {
  cpu_usage?: number
  memory_usage_percent?: number
  disk_usage_percent?: number
  load_one?: number
}

export interface DockerContainer {
  id: string
  image: string
  name: string
  status: string
  node_id: number
  created_at: string
}

export interface DockerImage {
  id: string
  repo_tags?: string[]
}

export interface DockerNetwork {
  id: string
  name: string
  driver: string
  scope: string
  internal: boolean
  attachable: boolean
  created: string | null
  ipam?: { driver?: string; config?: Array<{ subnet?: string; gateway?: string }> } | null
  containers?:
    | Array<{ name: string; ipv4_address: string | null; ipv6_address: string | null }>
    | null
}

export interface DockerVolume {
  name: string
  driver: string
  mountpoint: string
  created_at: string | null
  scope?: string
  labels?: Record<string, string> | null
  options?: Record<string, string> | null
}

export interface ComposeProject {
  name: string
  status: string
  config_files: string
}

export interface PruneResult {
  containers_deleted?: string[] | null
  networks_deleted?: string[] | null
  volumes_deleted?: string[] | null
  images_deleted?: unknown[] | null
  space_reclaimed?: number | null
  mode?: string
}

export interface NativeWebServerInfo {
  engine: string
  description: string
  installed: boolean
  package_installed: boolean
  version: string | null
  service_name: string | null
  running: boolean
  enabled: boolean
  binary_path: string | null
  config_path: string
  default_port: number
  listening_ports: number[]
}

export interface PluginResponse {
  id: string
  name: string
  version: string
  author: string
  description: string
  enabled: boolean
  status: string
  loaded_at: string
  last_executed_at: string | null
  exec_count: number
}

export interface OperationLog {
  id: number
  username: string
  action: string
  target: string | null
  ip: string | null
  created_at: string
}

export interface LogEntry {
  id: number
  source: string
  level: string
  message: string
  metadata: string | null
  created_at: string
}

export interface MetricsSnapshot {
  timestamp: number
  cpu_usage: number
  cpu_cores: number
  memory_usage_percent: number
  memory_total_mb: number
  memory_used_mb: number
  disk_usage_percent: number
  disk_total_gb: number
  disk_used_gb: number
  load_one: number
  load_five: number
  load_fifteen: number
  network_rx_mbps: number
  network_tx_mbps: number
}

export interface Memo {
  id: number
  content: string
  kind: string
  done: boolean
  created_at: string
  updated_at: string
}

export interface FileInfo {
  name: string
  path: string
  size: number
  is_dir: boolean
  permissions: string
  modified_at: string
  mime_type: string | null
}

export interface DatabaseInstance {
  id: number
  db_type: string
  name: string
  version: string
  port: number
  status: string
  install_path: string
  data_dir: string
  config_file: string
  root_user: string
  created_at: string
  updated_at: string
  resource_version: number
}

export interface FirewallRule {
  id: number
  name: string
  description: string | null
  protocol: string
  port: string | null
  source: string | null
  destination: string | null
  action: string
  enabled: boolean
  priority: number
  direction: string
  created_at: string
  updated_at: string
}

export interface FirewallStatus {
  backend: string
  backend_name: string
  status: string
}

export interface EngineInfo {
  name: string
  description: string
  default_port: number
  default_ssl_port: number
  supports_ssl: boolean
  supports_rewrite: boolean
  supports_reverse_proxy: boolean
  supports_load_balancing: boolean
}

export interface WebServerResponse {
  id: number
  engine: string
  version: string | null
  status: string
  config_path: string
  binary_path: string | null
  port: number
  created_at: string
  resource_version: number
}

export interface PerformancePresetInfo {
  name: string
  description: string
  recommended: boolean
  worker_processes: number
}

export interface PluginMetricsResponse {
  total_executions: number
  successful_executions: number
  failed_executions: number
  avg_execution_ms: number
  max_execution_ms: number
  min_execution_ms: number
  last_execution_ms: number
  peak_memory_bytes: number
}
