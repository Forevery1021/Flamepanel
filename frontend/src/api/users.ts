import api from './client'
import type { User } from '@/api/generated'
import type { Page } from '@/api/generated'

export function listUsers(page = 1, pageSize = 20) {
  return api.get<Page<User>>('/users', { params: { page, page_size: pageSize } })
}

export function createUser(username: string, password_hash: string, role: string) {
  return api.post<User>('/users', { username, password_hash, role })
}

export function updateUser(
  id: number,
  payload: { username: string; password_hash?: string; role: string },
) {
  return api.put<User>(`/users/${id}`, payload)
}

export function deleteUser(id: number) {
  return api.delete(`/users/${id}`)
}
