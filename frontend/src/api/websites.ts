import api from './client'
import type { Website } from '@/api/generated'
import type { Page } from '@/api/generated'

export function listWebsites(page = 1, pageSize = 20) {
  return api.get<Page<Website>>('/websites', { params: { page, page_size: pageSize } })
}

export function createWebsite(website: Website) {
  return api.post<number>('/websites', { website })
}

export function updateWebsite(id: number, website: Website) {
  return api.put<Website>(`/websites/${id}`, { website })
}

export function deleteWebsite(id: number) {
  return api.delete(`/websites/${id}`)
}

export function switchWebsiteEngine(id: number, engine: string) {
  return api.post<Website>(`/websites/${id}/switch-engine`, { engine })
}
