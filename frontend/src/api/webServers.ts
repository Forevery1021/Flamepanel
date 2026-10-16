import api, { longApi } from './client'
import type {
  WebServerResponse,
  EngineInfo,
  PerformancePresetInfo,
  NativeWebServerInfo,
  EngineCapabilitiesResponse,
  InstallCapabilitiesResult,
} from '@/types'
import type { Page } from '@/api/generated'

export function listEngines() {
  return api.get<EngineInfo[]>('/web-servers/engines')
}

export function listWebServers(page = 1, pageSize = 20) {
  return api.get<Page<WebServerResponse>>('/web-servers', {
    params: { page, page_size: pageSize },
  })
}

export function getWebServer(id: number) {
  return api.get<WebServerResponse>(`/web-servers/${id}`)
}

export function createWebServer(data: {
  engine: string
  version?: string
  config_path?: string
  binary_path?: string
  port?: number
}) {
  return api.post<WebServerResponse>('/web-servers', data)
}

export function updateWebServer(id: number, data: Partial<WebServerResponse>) {
  return api.put<WebServerResponse>(`/web-servers/${id}`, data)
}

export function deleteWebServer(id: number) {
  return api.delete(`/web-servers/${id}`)
}

export function startWebServer(id: number) {
  return api.post(`/web-servers/${id}/start`)
}

export function stopWebServer(id: number) {
  return api.post(`/web-servers/${id}/stop`)
}

export function restartWebServer(id: number) {
  return api.post(`/web-servers/${id}/restart`)
}

export function reloadWebServer(id: number) {
  return api.post(`/web-servers/${id}/reload`)
}

export function configtestWebServer(id: number) {
  return api.post(`/web-servers/${id}/configtest`)
}

/** 读取实例主配置（仅限 Web 引擎配置根下的文件；改配置走站点流水线 / 性能预设，不提供裸写接口） */
export function getWebServerConfig(id: number) {
  return api.get<{ id: number; config_path: string; content: string }>(
    `/web-servers/${id}/config`,
  )
}

export function switchWebServerEngine(id: number, engine: string) {
  // F-33: 引擎切换为长操作
  return longApi.post<WebServerResponse>(`/web-servers/${id}/switch-engine`, { engine })
}

export function applyWebServerPreset(id: number, preset: string) {
  return api.post(`/web-servers/${id}/preset`, { preset })
}

export function listPresets() {
  return api.get<PerformancePresetInfo[]>('/web-servers/presets')
}

// ── 原生控制（检测 / 安装 / 卸载 / 开机自启） ──
export function detectNativeWebServers() {
  return api.get<NativeWebServerInfo[]>('/web-servers/native/detect')
}

export function nativeInstallWebServer(engine: string, version?: string) {
  // F-33: 引擎安装为长操作
  return longApi.post<WebServerResponse>('/web-servers/native/install', { engine, version })
}

export function nativeUninstallWebServer(engine: string) {
  // F-33: 引擎卸载为长操作
  return longApi.post('/web-servers/native/uninstall', { engine })
}

export function nativeAutostartWebServer(engine: string, enabled: boolean) {
  return api.post('/web-servers/native/autostart', { engine, enabled })
}

export function setWebServerAutostart(id: number, enabled: boolean) {
  return api.post(`/web-servers/${id}/autostart`, { enabled })
}

export function nativeStatusWebServer(id: number) {
  return api.get<NativeWebServerInfo>(`/web-servers/${id}/native-status`)
}

// ── 配套能力（安装时"各取所需"） ──
export function getEngineCapabilities(engine: string) {
  return api.get<EngineCapabilitiesResponse>('/web-servers/capabilities', {
    params: { engine },
  })
}

export function installCapabilities(engine: string, capabilities: string[]) {
  // F-33: 配套依赖批量安装为长操作
  return longApi.post<InstallCapabilitiesResult>('/web-servers/capabilities/install', {
    engine,
    capabilities,
  })
}
