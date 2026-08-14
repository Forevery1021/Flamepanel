import api from './client'
import type {
  AppMetadata as GenAppMetadata,
  AppStoreListResponse as GenAppStoreListResponse,
  InstalledApp as GenInstalledApp,
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

export interface AppVersionInfo {
  version: string
  mode: string
  default_port?: number
  form_fields: AppFormField[]
  compose_template?: string
  native_scripts: string[]
  wasm_base64?: string
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

export function installApp(key: string, data: {
  package_key: string
  version?: string
  mode?: string
  name?: string
  port?: number
  container_name?: string
  values: Record<string, string>
  confirm_risky: boolean
}) {
  return api.post<InstalledApp>(`/app-store/packages/${key}/install`, data)
}

export function importPackage(path: string) {
  return api.post<AppMetadata>('/app-store/packages/import', { path })
}

export function batchImportPackages(paths: string[]) {
  return api.post<{ imported: AppMetadata[]; count: number }>(
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
  return api.post(`/app-store/installed/${id}/uninstall`)
}

export function upgradeApp(id: number, targetVersion?: string) {
  return api.post<InstalledApp>(`/app-store/installed/${id}/upgrade`, null, {
    params: targetVersion ? { target_version: targetVersion } : {},
  })
}

export function getAppLogs(id: number, tail = 200) {
  return api.get<{ logs: string }>(`/app-store/installed/${id}/logs`, {
    params: { tail },
  })
}

export function listWasmBuiltins() {
  return api.get<AppMetadata[]>('/app-store/wasm-builtins')
}
