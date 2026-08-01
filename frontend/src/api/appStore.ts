import api from './client'

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

export interface AppMetadata {
  key: string
  name: string
  category: string
  short_desc_zh: string
  short_desc_en?: string
  tags: string[]
  format: string
  modes: string[]
  versions: string[]
  default_version: string
  logo?: string
  min_memory_mb?: number
  architectures: string[]
  readme?: string
}

export interface InstalledApp {
  id: number
  package_key: string
  name: string
  version: string
  mode: string
  status: string
  access_url?: string
  install_path?: string
  container_name?: string
  port?: number
  params_json?: string
  created_at: string
  updated_at: string
}

export interface AppStoreListResponse {
  packages: AppMetadata[]
  total: number
}

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

export function listInstalledApps() {
  return api.get<InstalledApp[]>('/app-store/installed')
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
