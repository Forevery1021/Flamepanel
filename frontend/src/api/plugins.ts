import api from './client'
import type { MarketListResponse, PluginExecutionResponse, PluginResponse } from '@/types'

export function listPlugins() {
  return api.get<PluginResponse[]>('/plugins')
}

export function getPlugin(id: string) {
  return api.get<PluginResponse>(`/plugins/${id}`)
}

export function loadPlugin(
  id: string,
  name: string,
  wasm_base64: string,
  opts?: { version?: string; author?: string; description?: string; capabilities?: string[] },
) {
  return api.post<PluginResponse>('/plugins', { id, name, wasm_base64, ...opts })
}

export function unloadPlugin(id: string) {
  return api.post(`/plugins/${id}`)
}

export function enablePlugin(id: string) {
  return api.post<PluginResponse>(`/plugins/${id}/enable`)
}

export function disablePlugin(id: string) {
  return api.post<PluginResponse>(`/plugins/${id}/disable`)
}

/** 执行插件导出函数；input 经宿主 API `flame.input_read` 传给插件 */
export function executePlugin(id: string, func: string, input?: string) {
  return api.post<PluginExecutionResponse>(`/plugins/${id}/execute/${func}`, { input })
}

/** 整体替换插件的宿主能力授予 */
export function updatePluginCapabilities(id: string, capabilities: string[]) {
  return api.put<PluginResponse>(`/plugins/${id}/capabilities`, { capabilities })
}

/** 插件市场目录（内置 + 远程索引） */
export function listMarketPlugins() {
  return api.get<MarketListResponse>('/plugins/market')
}

/** 从市场安装；插件申请能力时须 acceptCapabilities=true */
export function installMarketPlugin(id: string, acceptCapabilities: boolean) {
  return api.post<PluginResponse>(`/plugins/market/${id}/install`, {
    accept_capabilities: acceptCapabilities,
  })
}

export function reloadPlugin(
  id: string,
  wasm_base64: string,
  memory_limit_bytes?: number,
  timeout_ms?: number,
) {
  return api.post(`/plugins/${id}/reload`, { wasm_base64, memory_limit_bytes, timeout_ms })
}

export function getPluginMetrics(id: string) {
  return api.get(`/plugins/${id}/metrics`)
}

export function resetPluginMetrics(id: string) {
  return api.delete(`/plugins/${id}/metrics`)
}

export function listPluginSettings(id: string) {
  return api.get(`/plugins/${id}/settings`)
}

export function setPluginSetting(id: string, key: string, value: string) {
  return api.post(`/plugins/${id}/settings`, { key, value })
}

export function getPluginSetting(id: string, key: string) {
  return api.get(`/plugins/${id}/settings/${key}`)
}
