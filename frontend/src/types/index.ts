// ─── System ──────────────────────────────────────────────────────────────────

export interface ServerInfo {
  cpu_usage: number
  cpu_cores: number
  memory_total_mb: number
  memory_used_mb: number
  memory_free_mb: number
  disk_total_gb: number
  disk_used_gb: number
  disk_free_gb: number
  uptime_seconds: number
  load_average: LoadAverage
  network: NetworkInfo
}

export interface LoadAverage {
  one: number
  five: number
  fifteen: number
}

export interface NetworkInfo {
  hostname: string
  interfaces: NetworkInterface[]
}

export interface NetworkInterface {
  name: string
  ipv4: string[]
  ipv6: string[]
  mac: string
}

export interface SystemInfoResponse {
  cpu_usage: number
  cpu_cores: number
  memory_total_mb: number
  memory_used_mb: number
  memory_free_mb: number
  memory_usage_percent: number
  disk_total_gb: number
  disk_used_gb: number
  disk_free_gb: number
  disk_usage_percent: number
  uptime_seconds: number
  uptime_display: string
  load_one: number
  load_five: number
  load_fifteen: number
  hostname: string
  network_interfaces: NetworkInterfaceResponse[]
}

export interface NetworkInterfaceResponse {
  name: string
  ipv4: string[]
  ipv6: string[]
  mac: string
}

export interface ProcessInfo {
  pid: number
  name: string
  cpu_usage: number
  memory_mb: number
  status: string
}

// ─── Dashboard ────────────────────────────────────────────────────────────────

export interface DashboardInfo {
  server_info: ServerInfo
  docker_containers_running: number
  docker_containers_total: number
  websites_running: number
  websites_total: number
  recent_logs: OperationLogEntry[]
  waf_rules_count: number
  waf_rules_enabled: number
}

export interface OperationLogEntry {
  username: string
  action: string
  target: string
  ip: string
  created_at: string
}

// ─── Docker ───────────────────────────────────────────────────────────────────

export interface DockerContainer {
  id: string
  name: string
  image: string
  status: string
  state: string
  ports: string[]
  created: string
}

export interface DockerImage {
  repository: string
  tag: string
  id: string
  size: string
  created: string
}

export interface ContainerActionRequest {
  id: string
  action: 'start' | 'stop' | 'restart'
}

// ─── File ─────────────────────────────────────────────────────────────────────

export interface FileItem {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: string
  permissions: string
}

// ─── Website ──────────────────────────────────────────────────────────────────

export interface Website {
  id: number
  domain: string
  root_path: string
  proxy_port: number | null
  ssl_enabled: boolean
  ssl_cert_path: string | null
  ssl_key_path: string | null
  config_path: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateWebsiteRequest {
  domain: string
  root_path?: string
  proxy_port?: number
  enable_ssl?: boolean
}

// ─── WAF ──────────────────────────────────────────────────────────────────────

export interface WafRule {
  id: number
  name: string
  pattern: string
  target: 'url' | 'header' | 'body' | 'cookie'
  action: 'block' | 'allow' | 'log'
  description: string | null
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateWafRuleRequest {
  name: string
  pattern: string
  target: string
  action: string
  description?: string
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

export interface LoginResponse {
  token: string
  user: {
    id: number
    username: string
    role: string
    created_at: string
    last_login: string | null
  }
}

export interface MessageResponse {
  success: boolean
  message: string
}

// ─── Terminal ─────────────────────────────────────────────────────────────────

export interface TerminalSession {
  id: string
}
