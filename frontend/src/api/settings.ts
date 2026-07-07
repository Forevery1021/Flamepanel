import api from './client'
import type { SettingEntry } from '@/types'

export function listSettings() {
  return api.get<SettingEntry[]>('/settings')
}

export function getSetting(key: string) {
  return api.get<SettingEntry>(`/settings/${key}`)
}

export function updateSetting(key: string, value: string) {
  return api.put('/settings', { key, value })
}
