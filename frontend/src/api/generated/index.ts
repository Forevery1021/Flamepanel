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
export type HealthDetail = Schema<'HealthDetail'>
export type ProcessEntry = Schema<'ProcessEntry'>
export type LoginRequest = Schema<'LoginRequest'>
export type LoginResponse = Schema<'LoginResponse'>
export type AppMetadata = Schema<'AppMetadata'>
export type AppStoreListResponse = Schema<'AppStoreListResponse'>
export type InstalledApp = Schema<'InstalledApp'>
export type InstalledAppResponse = Schema<'InstalledAppResponse'>
export type BackupEntry = Schema<'BackupEntryDto'>
export type PaginationParams = Schema<'PaginationParams'>

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
