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
  gpu_info: GpuInfo[]
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

export interface GpuInfo {
  name: string
  temperature_celsius: number
  utilization_percent: number
  memory_total_mb: number
  memory_used_mb: number
  memory_free_mb: number
  fan_speed_percent: number
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
  gpu_info: GpuInfo[]
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
  engine: string
  created_at: string
  updated_at: string
}

export interface CreateWebsiteRequest {
  domain: string
  root_path?: string
  proxy_port?: number
  enable_ssl?: boolean
  engine?: string
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

// ─── WAF IP Rules ──────────────────────────────────────────────────────────────

export interface WafIpRule {
  id: number
  ip: string
  action: 'allow' | 'block'
  description: string | null
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateWafIpRuleRequest {
  ip: string
  action: string
  description?: string
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

export interface LoginResponse {
  token: string
  username: string
  role: string
  expires_in: number
}

export interface MessageResponse {
  success: boolean
  message: string
}

// ─── User ──────────────────────────────────────────────────────────────────────

export interface User {
  id: number
  username: string
  role: string
  created_at: string
  last_login: string | null
}

// ─── Terminal ─────────────────────────────────────────────────────────────────

export interface TerminalSession {
  id: string
}

// ─── Cleanup ──────────────────────────────────────────────────────────────────

export interface CleanupItem {
  category: string
  name: string
  description: string
  path: string
  size_bytes: number
  size_display: string
  can_clean: boolean
}

export interface CleanupScanResult {
  items: CleanupItem[]
  total_bytes: number
  total_display: string
}

export interface CleanupRequest {
  categories: string[]
}

export interface CleanupResult {
  cleaned_items: string[]
  freed_bytes: number
  freed_display: string
  errors: string[]
}

// ─── Settings ──────────────────────────────────────────────────────────────────

export interface PanelSettings {
  theme: string
  language: string
  theme_color?: string
  background_image?: string
  background_opacity?: number
}

export interface UpdateSettingsRequest {
  theme?: string
  language?: string
  theme_color?: string
  background_image?: string
  background_opacity?: number
}

// ─── Cron ──────────────────────────────────────────────────────────────────────

export interface CronJob {
  id: number
  name: string
  schedule: string
  command: string | null
  url: string | null
  enabled: boolean
  last_run: string | null
  next_run: string | null
  created_at: string
  updated_at: string
}

export interface CreateCronJobRequest {
  name: string
  schedule: string
  command?: string
  url?: string
}

export interface UpdateCronJobRequest {
  name?: string
  schedule?: string
  command?: string
  url?: string
  enabled?: boolean
}

export interface CronJobLog {
  id: number
  job_id: number
  status: string
  output: string | null
  started_at: string
  finished_at: string | null
}

// ─── Database ──────────────────────────────────────────────────────────────────

export interface DatabaseInstance {
  id: number
  name: string
  db_type: string
  version: string
  port: number
  container_id: string | null
  username: string
  password: string
  status: string
  data_dir: string | null
  created_at: string
  updated_at: string
}

export interface CreateDatabaseRequest {
  name: string
  db_type: string
  version?: string
  port?: number
  password: string
}

export interface DatabaseBackup {
  id: number
  instance_id: number
  filename: string
  size_bytes: number
  created_at: string
}

// ─── App Store ─────────────────────────────────────────────────────────────────

export interface AppManifest {
  key: string
  name: string
  category: string
  description: string
  version: string
  default_port: number
  icon: string
  compose: string
}

export interface InstalledApp {
  id: number
  app_key: string
  name: string
  category: string
  port: number
  status: string
  compose_file: string | null
  data_dir: string | null
  version: string
  description: string | null
  created_at: string
  updated_at: string
}

export interface InstallAppRequest {
  app_key: string
  name: string
  port?: number
  extra_env?: Record<string, string>
}

// ─── AI Assistant ─────────────────────────────────────────────────────────────

export interface AiConversation {
  id: number
  title: string
  model: string
  messages: string
  created_at: string
  updated_at: string
}

export interface AiMessage {
  role: string
  content: string
}

export interface AiChatRequest {
  conversation_id?: number
  model: string
  message: string
}

export interface AiChatResponse {
  conversation_id: number
  title: string
  reply: string
}

export interface AiModelInfo {
  name: string
  size: string
  modified: string
}

export interface AiAnalyzeRequest {
  log_content: string
  model?: string
}

// ─── Backup ────────────────────────────────────────────────────────────────────

export interface BackupConfig {
  id: number
  name: string
  backup_type: string
  target_path: string
  storage_type: string
  storage_path: string
  cron_expr: string | null
  retention_days: number
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateBackupConfigRequest {
  name: string
  backup_type: string
  target_path: string
  storage_type?: string
  storage_path?: string
  cron_expr?: string
  retention_days?: number
}

export interface UpdateBackupConfigRequest {
  name?: string
  backup_type?: string
  target_path?: string
  storage_type?: string
  storage_path?: string
  cron_expr?: string
  retention_days?: number
  enabled?: boolean
}

export interface BackupRecord {
  id: number
  config_id: number
  file_name: string
  file_size: number
  status: string
  error_message: string | null
  started_at: string
  finished_at: string | null
}

// ─── Nodes ─────────────────────────────────────────────────────────────────────

export interface NodeInfo {
  id: number
  name: string
  host: string
  agent_port: number
  auth_token: string
  status: string
  cpu_usage: number
  memory_usage_percent: number
  disk_usage_percent: number
  load_one: number
  last_heartbeat: string
  created_at: string
  updated_at: string
}

// ─── MCP / Skills ──────────────────────────────────────────────────────────────

export interface ToolInfo {
  name: string
  description: string
  parameters: Record<string, any>
}

export interface ToolCallRequest {
  name: string
  arguments?: Record<string, any>
}

export interface ToolCallResponse {
  name: string
  result: string
}

// ─── Alerts ────────────────────────────────────────────────────────────────────

export interface NotificationChannel {
  id: number
  name: string
  channel_type: string
  config: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateNotificationChannelRequest {
  name: string
  channel_type: string
  config: Record<string, any>
}

export interface UpdateNotificationChannelRequest {
  name?: string
  channel_type?: string
  config?: Record<string, any>
  enabled?: boolean
}

export interface AlertRule {
  id: number
  name: string
  metric_type: string
  condition: string
  threshold: number
  duration_seconds: number
  channel_ids: string
  enabled: boolean
  cooldown_minutes: number
  last_triggered: string | null
  created_at: string
  updated_at: string
}

export interface CreateAlertRuleRequest {
  name: string
  metric_type: string
  condition: string
  threshold: number
  duration_seconds?: number
  channel_ids: number[]
  cooldown_minutes?: number
}

export interface UpdateAlertRuleRequest {
  name?: string
  metric_type?: string
  condition?: string
  threshold?: number
  duration_seconds?: number
  channel_ids?: number[]
  enabled?: boolean
  cooldown_minutes?: number
}

export interface AlertHistory {
  id: number
  rule_id: number
  rule_name: string
  metric_type: string
  metric_value: number
  threshold: number
  status: string
  message: string
  created_at: string
}

// ─── RBAC ─────────────────────────────────────────────────────────────────────

export interface Role {
  id: number
  name: string
  description: string
  is_system: boolean
  created_at: string
  updated_at: string
}

export interface Permission {
  id: number
  name: string
  resource: string
  action: string
  description: string
}

export interface RoleWithPermissions {
  id: number
  name: string
  description: string
  is_system: boolean
  permissions: Permission[]
  created_at: string
  updated_at: string
}

export interface CreateRoleRequest {
  name: string
  description?: string
  permission_ids: number[]
}

export interface UpdateRoleRequest {
  name?: string
  description?: string
  permission_ids?: number[]
}

export interface AssignRoleRequest {
  user_id: number
  role: string
}

export interface MyPermissionsResponse {
  role: string
  username: string
  permissions: string[]
}
