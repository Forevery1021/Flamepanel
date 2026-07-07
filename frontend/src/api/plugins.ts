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
