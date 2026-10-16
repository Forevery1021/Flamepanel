/**
 * F4.1 OpenAPI 生成类型 — 统一出口。
 *
 * 数据源：后端 `GET /api/openapi.json`（utoipa 编译期生成，见 flame-kernel/src/openapi.rs）。
 * 生成命令：`npm run openapi:generate`（读取 frontend/openapi.json 快照，输出本目录 openapi.ts）。
 *
 * 规范：
 * - 后端已覆盖的实体类型一律从本模块导入（业务禁止手写重复定义，避免漂移）
 * - 后端未覆盖的端点类型保留在 `@/types` 或各 api 模块内手写
 */
import type { components, operations, paths } from './openapi'

export type { components, operations, paths }

type Schemas = components['schemas']

/** 按名称取 OpenAPI 组件 Schema 类型（处理 `PaginatedResponse<T>` 等带泛型名的 schema） */
export type Schema<K extends keyof Schemas> = Schemas[K]

/* ── 实体 Schema 具名导出（后端 OpenAPI 已覆盖） ───────────── */
export type User = Schema<'User'>
export type ServerNode = Schema<'ServerNode'>
export type Website = Schema<'Website'>
export type ScheduledTask = Schema<'ScheduledTask'>
export type SettingEntry = Schema<'SettingEntry'>
export type AccessInfo = Schema<'AccessInfo'>
export type HealthDetail = Schema<'HealthDetail'>
export type ProcessEntry = Schema<'ProcessEntry'>
export type LoginRequest = Schema<'LoginRequest'>
export type LoginResponse = Schema<'LoginResponse'>
export type LoginTwoFactorRequest = Schema<'LoginTwoFactorRequest'>
export type TwoFactorStatusResponse = Schema<'TwoFactorStatusResponse'>
export type TwoFactorSetup = Schema<'TwoFactorSetup'>
export type RecoveryCodesResponse = Schema<'RecoveryCodesResponse'>
export type AppMetadata = Schema<'AppMetadata'>
export type AppStoreListResponse = Schema<'AppStoreListResponse'>
export type AppSource = Schema<'AppSource'>
export type StoreSecurityConfig = Schema<'StoreSecurityConfig'>
export type UpdateStoreSecurityRequest = Schema<'UpdateStoreSecurityRequest'>
export type Certificate = Schema<'Certificate'>
export type CertificateRenewalHistory = Schema<'CertificateRenewalHistory'>
export type RenewalEntry = Schema<'RenewalEntry'>
export type RenewalEventKind = Schema<'RenewalEventKind'>
export type InstalledApp = Schema<'InstalledApp'>
export type InstalledAppResponse = Schema<'InstalledAppResponse'>
export type UpgradePreview = Schema<'UpgradePreview'>
export type ComposeMerge = Schema<'ComposeMerge'>
export type MergeConflict = Schema<'MergeConflict'>
export type InstallMongodbRequest = Schema<'InstallMongodbRequest'>
export type BackupEntry = Schema<'BackupEntryDto'>
export type PaginationParams = Schema<'PaginationParams'>
export type WafRule = Schema<'WafRule'>
export type WafLog = Schema<'WafLog'>
export type WafEngineStatus = Schema<'WafEngineStatus'>
export type WafApplyReport = Schema<'WafApplyReport'>
export type WafEngineApply = Schema<'WafEngineApply'>
export type WafIngestStatus = Schema<'WafIngestStatus'>
/* S-06 顶栏概览聚合 */
export type OverviewBadges = Schema<'OverviewBadges'>
export type NodeBadges = Schema<'NodeBadges'>
export type ContainerBadges = Schema<'ContainerBadges'>
export type TaskBadges = Schema<'TaskBadges'>
export type AlertBadges = Schema<'AlertBadges'>
/* S-07 监控历史 */
export type MetricsHistoryPoint = Schema<'MetricsHistoryPoint'>
export type MetricsHistoryResponse = Schema<'MetricsHistoryResponse'>
/** S-17 系统体检（`GET /api/inspection`） */
export type InspectionReport = Schema<'InspectionReport'>
export type InspectionSummary = Schema<'InspectionSummary'>
export type KernelInfo = Schema<'KernelInfo'>
export type Finding = Schema<'Finding'>
export type Severity = Schema<'Severity'>
export type CheckStatus = Schema<'CheckStatus'>
export type Category = Schema<'Category'>
/** S-12 系统工具箱 · 进程管理（`/api/toolbox/processes`） */
export type ProcessInfo = Schema<'ProcessInfo'>
export type ProcessRow = Schema<'ProcessRow'>
// W0 内核日志
export type LogLevel = Schema<'LogLevel'>
export type KernelLogEntry = Schema<'KernelLogEntry'>
export type LevelCounts = Schema<'LevelCounts'>
export type LogFileInfo = Schema<'LogFileInfo'>
export type TransferItem = Schema<'TransferItem'>
export type TransferMode = Schema<'TransferMode'>
export type TransferRequest = Schema<'TransferRequest'>
export type TransferResult = Schema<'TransferResult'>
export type LogPage = Schema<'LogPage'>
export type KernelLogOverview = Schema<'KernelLogOverview'>
export type KernelLogStats = Schema<'KernelLogStats'>
export type ProcessListResponse = Schema<'ProcessListResponse'>
export type ProtectedReason = Schema<'ProtectedReason'>
export type ProcessSignal = Schema<'ProcessSignal'>
export type KillProcessRequest = Schema<'KillProcessRequest'>
export type KillProcessResponse = Schema<'KillProcessResponse'>
export type HostDetails = Schema<'HostDetails'>
export type HostSystem = Schema<'HostSystem'>
export type HostCpu = Schema<'HostCpu'>
export type HostMemory = Schema<'HostMemory'>
export type HostTime = Schema<'HostTime'>
export type DnsConfig = Schema<'DnsConfig'>
export type HostsEntry = Schema<'HostsEntry'>
export type HostInterface = Schema<'HostInterface'>
export type HostDisk = Schema<'HostDisk'>
export type PanelRuntime = Schema<'PanelRuntime'>
export type CreateWafRuleRequest = Schema<'CreateWafRuleRequest'>
export type UpdateWafRuleRequest = Schema<'UpdateWafRuleRequest'>

/* ── 请求体 Schema（OpenAPI 已覆盖的写操作） ───────────────── */
export type CreateUserRequest = Schema<'CreateUserRequest'>
export type UpdateUserRequest = Schema<'UpdateUserRequest'>
export type CreateNodeRequest = Schema<'CreateNodeRequest'>
export type CreateWebsiteRequest = Schema<'CreateWebsiteRequest'>
export type UpdateSettingRequest = Schema<'UpdateSettingRequest'>
export type CreateTaskRequest = Schema<'CreateTaskRequest'>
export type UpdateTaskRequest = Schema<'UpdateTaskRequest'>
export type RemoteExecRequest = Schema<'RemoteExecRequest'>
export type RemoteBatchExecRequest = Schema<'RemoteBatchExecRequest'>
export type RemoteUploadRequest = Schema<'RemoteUploadRequest'>
export type HeartbeatRequest = Schema<'HeartbeatRequest'>

/* ── 分页响应容器（与后端 PaginatedResponse<T> 同构） ─────── */
export interface Page<T> {
  data: T[]
  page: number
  page_size: number
  total: number
  total_pages: number
}
export type DnsProviderAccountDto = Schema<'DnsProviderAccountDto'>
export type DnsProviderAccount = DnsProviderAccountDto
export type BackupAccountDto = Schema<'AccountDto'>
export type BackupAccount = Schema<'BackupAccount'>
export type CreateAccountRequest = Schema<'CreateAccountRequest'>
export type UpdateAccountRequest = Schema<'UpdateAccountRequest'>
export type RemoteObjectDto = Schema<'RemoteObjectDto'>
export type BackupJobDto = Schema<'JobDto'>
export type CreateJobRequest = Schema<'CreateJobRequest'>
export type UpdateJobRequest = Schema<'UpdateJobRequest'>
export type RestoreFromCloudResponse = Schema<'RestoreFromCloudResponse'>

/* ── Runtime / Project（M-B 主线二 Stage A-3） ─────────────── */
export type ProjectDto = Schema<'ProjectDto'>
export type CreateProjectRequest = Schema<'CreateProjectRequest'>
export type UpdateProjectRequest = Schema<'UpdateProjectRequest'>
export type RuntimeDto = Schema<'RuntimeDto'>
export type InstallRuntimeRequest = Schema<'InstallRuntimeRequest'>
export type RuntimeVersionInfo = Schema<'RuntimeVersionInfo'>
export type ProcessStatus = Schema<'ProcessStatus'>
export type LogLine = Schema<'LogLine'>
// Webhook 部署通道（M-B 主线二 Stage A-7）
export type WebhookDeployDto = Schema<'WebhookDeployDto'>
// Procfile 进程模型（M-B 主线二 Stage A-8 · Doc/20 §5.8 ③）
export type ProcfileDto = Schema<'ProcfileDto'>
// 零停机切换（M-B 主线二 Stage A-9 · Doc/20 §5.8 ④）
export type HealthCheckDto = Schema<'HealthCheckDto'>
export type ProcEntry = Schema<'ProcEntry'>
// 构建器自动协商（M-B 主线二 Stage A-8 · Doc/20 §5.8 ②）
export type BuilderDecisionDto = Schema<'BuilderDecisionDto'>
// 服务联链（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
export type ProjectDbLinkDto = Schema<'ProjectDbLinkDto'>
export type LinkableDatabaseDto = Schema<'LinkableDatabaseDto'>
export type LinkDatabaseRequest = Schema<'LinkDatabaseRequest'>

/* ── M-AI-1：MCP / AI 接入中心 ─────────────────────────────── */
export type AgentPrincipal = Schema<'AgentPrincipal'>
export type PrincipalView = Schema<'PrincipalView'>
export type AgentSession = Schema<'AgentSession'>
export type ToolCallRecord = Schema<'ToolCallRecord'>
export type ToolInfo = Schema<'ToolInfo'>
export type McpPromptArgument = Schema<'McpPromptArgument'>
export type McpPromptInfo = Schema<'McpPromptInfo'>
export type McpPromptRendered = Schema<'McpPromptRendered'>
export type AssistantConfigView = Schema<'AssistantConfigView'>
export type AssistantConfigPatch = Schema<'AssistantConfigPatch'>
export type AssistantChatTurn = Schema<'ChatTurn'>
export type AssistantChatResponse = Schema<'ChatResponse'>
export type AssistantToolStep = Schema<'ToolStep'>
export type AssistantTestResult = Schema<'TestResult'>
export type ToolApproval = Schema<'ToolApproval'>
export type CreatePrincipalRequest = Schema<'CreatePrincipalRequest'>
export type CreatePrincipalResponse = Schema<'CreatePrincipalResponse'>
