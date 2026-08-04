import api from './client'
import type { LoginResponse } from '@/types'

export function login(username: string, password: string) {
  return api.post<LoginResponse>('/auth/login', { username, password })
}

export function refreshToken() {
  return api.post<LoginResponse>('/auth/refresh')
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
