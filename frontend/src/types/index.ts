export interface User {
  id: number
  username: string
  password_hash: string
  role: string
  created_at: string
}

export interface ServerNode {
  id: number
  name: string
  hostname: string
  ip_address: string
  status: string
  created_at: string
}

export interface Website {
  id: number
  name: string
  domain: string
  root_path: string
  status: string
  node_id: number
  engine: string
  ssl_enabled: boolean
  proxy_enabled: boolean
  proxy_pass: string | null
  created_at: string
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
}

export interface LoginResponse {
  token: string
  username: string
  role: string
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
}

export interface SettingEntry {
  key: string
  value: string
  description: string
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

export interface PaginatedResponse<T> {
  data: T[]
  page: number
  page_size: number
  total: number
  total_pages: number
}
