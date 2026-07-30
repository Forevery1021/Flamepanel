import api from './client'
import type { WebServerResponse, EngineInfo, PaginatedResponse } from '@/types'

export function listEngines() {
  return api.get<EngineInfo[]>('/web-servers/engines')
}

export function listWebServers(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<WebServerResponse>>('/web-servers', { params: { page, page_size: pageSize } })
}

export function getWebServer(id: number) {
  return api.get<WebServerResponse>(`/web-servers/${id}`)
}

export function createWebServer(data: { engine: string; version?: string; config_path?: string; binary_path?: string; port?: number }) {
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

export function getWebServerConfig(id: number) {
  return api.get<string>(`/web-servers/${id}/config`)
}

export function updateWebServerConfig(id: number, config: string) {
  return api.post(`/web-servers/${id}/config`, { config })
}
