import api from './client'
import type { Website } from '@/types'

export function listWebsites() {
  return api.get<Website[]>('/websites')
}

export function createWebsite(website: Website) {
  return api.post<number>('/websites', { website })
}
