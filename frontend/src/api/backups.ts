import api, { longApi } from './client'

export interface BackupEntry {
  filename: string
  size: number
  created_at: string
}

export function listBackups() {
  return api.get<BackupEntry[]>('/backups')
}

export function createBackup() {
  return api.post<BackupEntry>('/backups')
}

export async function downloadBackup(filename: string) {
  const res = await api.get<Blob>(`/backups/${encodeURIComponent(filename)}`, {
    responseType: 'blob',
  })
  const url = URL.createObjectURL(res.data)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

export function restoreBackup(filename: string) {
  // F-33: 恢复备份可能耗时较长
  return longApi.post(`/backups/${encodeURIComponent(filename)}/restore`, { filename })
}

export function deleteBackup(filename: string) {
  return api.delete(`/backups/${encodeURIComponent(filename)}`)
}
