import { defineStore } from 'pinia'
import api from '@/api/client'
import type { LoginResponse } from '@/types'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('token') || '',
    username: localStorage.getItem('username') || '',
    role: localStorage.getItem('role') || '',
  }),
  getters: {
    isLoggedIn: (state) => !!state.token,
  },
  actions: {
    async login(username: string, password: string) {
      const res = await api.post<LoginResponse>('/auth/login', { username, password })
      const { token, username: uname, role } = res.data
      this.token = token
      this.username = uname
      this.role = role
      localStorage.setItem('token', token)
      localStorage.setItem('username', uname)
      localStorage.setItem('role', role)
    },
    logout() {
      this.token = ''
      this.username = ''
      this.role = ''
      localStorage.clear()
    },
  },
})
