import api from './client'
import type { User, PaginatedResponse } from '@/types'

export function listUsers(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<User>>('/users', { params: { page, page_size: pageSize } })
}

export function createUser(username: string, password_hash: string, role: string) {
  return api.post<User>('/users', { username, password_hash, role })
}

export function deleteUser(id: number) {
  return api.delete(`/users/${id}`)
}
