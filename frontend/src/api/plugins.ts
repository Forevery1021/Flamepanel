import api from './client'
import type { PluginResponse } from '@/types'

export function listPlugins() {
  return api.get<PluginResponse[]>('/plugins')
}

export function getPlugin(id: string) {
  return api.get<PluginResponse>(`/plugins/${id}`)
}

export function loadPlugin(id: string, name: string, wasm_base64: string, opts?: { version?: string; author?: string; description?: string }) {
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

export function executePlugin(id: string, func: string, args?: number[]) {
  return api.post(`/plugins/${id}/execute/${func}`, { args })
}

export function reloadPlugin(id: string, wasm_base64: string, memory_limit_bytes?: number, timeout_ms?: number) {
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
