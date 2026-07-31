import api from './client'
import type { SettingEntry, PaginatedResponse } from '@/types'

export function listSettings(page = 1, pageSize = 50) {
  return api.get<PaginatedResponse<SettingEntry>>('/settings', {
    params: { page, page_size: pageSize },
  })
}

export function getSetting(key: string) {
  return api.get<SettingEntry>(`/settings/${key}`)
}

export function updateSetting(key: string, value: string) {
  return api.put('/settings', { key, value })
}
