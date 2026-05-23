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
      const { token, user } = res.data
      this.token = token
      this.username = user.username
      this.role = user.role
      localStorage.setItem('token', token)
      localStorage.setItem('username', user.username)
      localStorage.setItem('role', user.role)
    },
    logout() {
      this.token = ''
      this.username = ''
      this.role = ''
      localStorage.clear()
    },
  },
})
