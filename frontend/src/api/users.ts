import api from './client'
import type { User } from '@/types'

export function listUsers() {
  return api.get<User[]>('/users')
}

export function createUser(username: string, password_hash: string, role: string) {
  return api.post<User>('/users', { username, password_hash, role })
}
