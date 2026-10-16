import api, { longApi } from './client'
import type {
  AppMetadata as GenAppMetadata,
  AppStoreListResponse as GenAppStoreListResponse,
  InstalledApp as GenInstalledApp,
  StoreSecurityConfig as GenStoreSecurityConfig,
  UpgradePreview,
} from '@/api/generated'

export interface AppFormField {
  env_key: string
  label_zh: string
  label_en?: string
  field_type: string
  default?: string
  required: boolean
  pattern?: string
  min?: number
  max?: number
  min_length?: number
  max_length?: number
  options: Array<{ label: string; value: string }>
  description?: string
  group?: string
}

export interface AppHooks {
  pre_install?: string
  post_install?: string
  pre_upgrade?: string
  post_upgrade?: string
  pre_uninstall?: string
}

export interface AppVersionInfo {
  version: string
  mode: string
  default_port?: number
  form_fields: AppFormField[]
  compose_template?: string
  native_scripts: string[]
  wasm_base64?: string
  /** 生命周期钩子脚本（Doc/20 §4.3，相对包根路径） */
  hooks?: AppHooks
  min_memory_mb?: number
  architectures: string[]
}

/** 应用元数据（后端 OpenAPI 已覆盖：src/api/generated） */
export type AppMetadata = GenAppMetadata

/** 已安装应用（后端 OpenAPI 已覆盖） */
export type InstalledApp = GenInstalledApp

/** 应用商店列表响应（后端 OpenAPI 已覆盖） */
export type AppStoreListResponse = GenAppStoreListResponse

export function listPackages(category?: string) {
  return api.get<AppStoreListResponse>('/app-store/packages', {
    params: category ? { category } : {},
  })
}

export function getPackage(key: string) {
  return api.get<AppMetadata>(`/app-store/packages/${key}`)
}

export function getPackageVersion(key: string, version: string) {
  return api.get<AppVersionInfo>(`/app-store/packages/${key}/versions/${version}`)
}

export function installApp(
  key: string,
  data: {
    package_key: string
    version?: string
    mode?: string
    name?: string
    port?: number
    container_name?: string
    values: Record<string, string>
    confirm_risky: boolean
    /** 含生命周期钩子脚本时需显式确认执行宿主机脚本（Doc/20 §4.3） */
    acknowledge_scripts?: boolean
  },
) {
  // F-33: 长操作不设超时（0），安装任务由后端异步执行 + 前端轮询进度
  return longApi.post<InstalledApp>(`/app-store/packages/${key}/install`, data)
}

export function importPackage(path: string) {
  // F-33: 导入大文件可能超过 15s
  return longApi.post<AppMetadata>('/app-store/packages/import', { path })
}

export function batchImportPackages(paths: string[]) {
  // F-33: 批量导入为长操作
  return longApi.post<{ imported: AppMetadata[]; count: number }>(
    '/app-store/packages/batch-import',
    { paths },
  )
}

export function listInstalledApps() {
  return api.get<InstalledApp[]>('/app-store/installed')
}

export function launchApp(id: number) {
  return api.post<InstalledApp>(`/app-store/installed/${id}/launch`)
}

export function getInstalledApp(id: number) {
  return api.get(`/app-store/installed/${id}`)
}

export function uninstallApp(id: number) {
  // F-33: 卸载也是长操作
  return longApi.post(`/app-store/installed/${id}/uninstall`)
}

/** compose 冲突处理：abort（默认，冲突即 409）/ local（保留本地）/ upstream（采用新版本）。 */
export type ConflictStrategy = 'abort' | 'local' | 'upstream'

export function upgradeApp(id: number, targetVersion?: string, strategy?: ConflictStrategy) {
  const params: Record<string, string> = {}
  if (targetVersion) params.target_version = targetVersion
  if (strategy && strategy !== 'abort') params.conflict_strategy = strategy
  return api.post<InstalledApp>(`/app-store/installed/${id}/upgrade`, null, { params })
}

// 预览契约直接取自 OpenAPI 生成类型，避免与后端漂移
export type { ComposeMerge, MergeConflict, UpgradePreview } from '@/api/generated'

/** 升级预览：目标版本 compose 与本地三方合并（只读，不改文件）。 */
export function upgradePreview(id: number, targetVersion?: string) {
  return api.get<UpgradePreview>(`/app-store/installed/${id}/upgrade-preview`, {
    params: targetVersion ? { target_version: targetVersion } : {},
  })
}

// ── 原生软件生命周期（Doc/16 §6.5：status/start/stop/restart） ──

export function nativeAppStatus(id: number) {
  return api.get<{ id: number; status: string }>(`/app-store/installed/${id}/native-status`)
}

export function nativeAppStart(id: number) {
  return api.post(`/app-store/installed/${id}/native-start`)
}

export function nativeAppStop(id: number) {
  return api.post(`/app-store/installed/${id}/native-stop`)
}

export function nativeAppRestart(id: number) {
  return api.post(`/app-store/installed/${id}/native-restart`)
}

export function getAppLogs(id: number, tail = 200) {
  return api.get<{ logs: string }>(`/app-store/installed/${id}/logs`, {
    params: { tail },
  })
}

export function listWasmBuiltins() {
  return api.get<AppMetadata[]>('/app-store/wasm-builtins')
}

// ── 远程应用源（M-A Issue 1.2/1.4，Doc/20 §4.1） ──

import type { AppSource as GenAppSource } from '@/api/generated'

/** 远程应用源（后端 OpenAPI 已覆盖） */
export type AppSource = GenAppSource

export interface SourcesResponse {
  total: number
  sources: AppSource[]
}

export interface CreateSourceRequest {
  name: string
  source_type: string
  url: string
  public_key?: string | null
  /** 未配置公钥的源须显式确认（Doc/20 §4.2：未签名第三方源二次确认） */
  confirm_unsigned?: boolean
}

export interface UpdateSourceRequest {
  name?: string
  url?: string
  enabled?: boolean
  public_key?: string | null
  /** 清空公钥（签名源降级为未签名源）须显式确认 */
  confirm_unsigned?: boolean
}

export interface SyncSourceResponse {
  added: number
  skipped: number
  packages: string[]
}

export function listSources() {
  return api.get<SourcesResponse>('/app-store/sources')
}

export function createSource(data: CreateSourceRequest) {
  // F-33: 添加源会立即触发索引同步（长操作）
  return longApi.post<AppSource>('/app-store/sources', data)
}

export function updateSource(id: number, data: UpdateSourceRequest) {
  return api.put<AppSource>(`/app-store/sources/${id}`, data)
}

export function deleteSource(id: number) {
  return api.delete<{ success: boolean }>(`/app-store/sources/${id}`)
}

export function syncSource(id: number) {
  // F-33: 同步大仓库为长操作
  return longApi.post<SyncSourceResponse>(`/app-store/sources/${id}/sync`)
}

export function listSourcePackages(id: number) {
  return api.get<AppStoreListResponse>(`/app-store/sources/${id}/packages`)
}

// ── 商店安全配置（Doc/20 §4.2 收口） ──

/** 商店安全配置（后端 OpenAPI 已覆盖） */
export type StoreSecurityConfig = GenStoreSecurityConfig

export interface UpdateStoreSecurityRequest {
  /** optional（源配公钥才验签）| required（远程包强制验签） */
  signature_policy?: string
  /** 镜像仓库白名单追加项（全量替换） */
  registry_whitelist?: string[]
}

export function getStoreSecurity() {
  return api.get<StoreSecurityConfig>('/app-store/security')
}

export function updateStoreSecurity(data: UpdateStoreSecurityRequest) {
  return api.put<StoreSecurityConfig>('/app-store/security', data)
}
