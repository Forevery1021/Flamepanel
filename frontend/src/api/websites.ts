import api from './client'
import type { Website, PaginatedResponse } from '@/types'

export function listWebsites(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<Website>>('/websites', { params: { page, page_size: pageSize } })
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
