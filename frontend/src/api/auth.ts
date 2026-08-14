import api from './client'
import { STORAGE_KEYS } from '@/utils/storage'
import type { LoginResponse } from '@/api/generated'

export function login(username: string, password: string) {
  return api.post<LoginResponse>('/auth/login', { username, password })
}

export function refreshToken() {
  // 刷新必须使用 Refresh Token（access token 不可用于刷新）
  const refresh = localStorage.getItem(STORAGE_KEYS.refreshToken) || ''
  return api.post<LoginResponse>('/auth/refresh', undefined, {
    headers: { Authorization: `Bearer ${refresh}` },
  })
}

export function fetchMe() {
  return api.get<{
    id: number
    username: string
    role: string
    must_change_password: boolean
  }>('/auth/me')
}

export function changePassword(old_password: string, new_password: string) {
  return api.post('/auth/change-password', { old_password, new_password })
}
