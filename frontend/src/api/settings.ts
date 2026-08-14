import api from './client'
import type { SettingEntry } from '@/api/generated'
import type { Page } from '@/api/generated'

export function listSettings(page = 1, pageSize = 50) {
  return api.get<Page<SettingEntry>>('/settings', {
    params: { page, page_size: pageSize },
  })
}

export function getSetting(key: string) {
  return api.get<SettingEntry>(`/settings/${key}`)
}

export function updateSetting(key: string, value: string) {
  return api.put('/settings', { key, value })
}

/** 批量原子写设置：多键在一次事务内全部写入。 */
export function updateSettingsBatch(settings: [string, string][]) {
  return api.patch('/settings/batch', { settings })
}
