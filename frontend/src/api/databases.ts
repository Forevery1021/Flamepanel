import api from './client'
import type { DatabaseInstance } from '@/types'

export function listDatabases() {
  return api.get<DatabaseInstance[]>('/databases')
}

export function getDatabase(id: number) {
  return api.get<DatabaseInstance>(`/databases/${id}`)
}

export function deleteDatabase(id: number) {
  return api.delete(`/databases/${id}`)
}

export function installMysql(data: { name: string; version?: string; port?: number; root_password?: string }) {
  return api.post<DatabaseInstance>('/databases/mysql/install', data)
}

export function installRedis(data: { name: string; version?: string; port?: number; password?: string }) {
  return api.post<DatabaseInstance>('/databases/redis/install', data)
}

export function startDatabase(id: number) {
  return api.post(`/databases/${id}/start`)
}

export function stopDatabase(id: number) {
  return api.post(`/databases/${id}/stop`)
}

export function restartDatabase(id: number) {
  return api.post(`/databases/${id}/restart`)
}

export function checkDatabaseStatus(id: number) {
  return api.get<string>(`/databases/${id}/status`)
}

export function listInternalDatabases(id: number) {
  return api.get<string[]>(`/databases/${id}/databases`)
}

export function createInternalDatabase(id: number, name: string, charset?: string) {
  return api.post(`/databases/${id}/databases`, { name, charset })
}

export function dropInternalDatabase(id: number, dbName: string) {
  return api.delete(`/databases/${id}/databases/${dbName}`)
}

export function createDatabaseUser(id: number, username: string, password: string, host?: string) {
  return api.post(`/databases/${id}/users`, { username, password, host })
}

export function dropDatabaseUser(id: number, username: string) {
  return api.delete(`/databases/${id}/users/${username}`)
}

export function uninstallDatabase(id: number) {
  return api.post(`/databases/${id}/uninstall`)
}
