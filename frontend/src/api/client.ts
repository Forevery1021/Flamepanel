import axios, { AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { refreshToken } from './auth'

/** 后端统一错误响应（见 flame-kernel/src/core/error.rs） */
export interface ApiError {
  code: string
  message: string
  status: number
}

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

const TOKEN_KEYS = ['token', 'username', 'role'] as const

function persistAuth(token: string, username?: string, role?: string) {
  localStorage.setItem('token', token)
  if (username !== undefined) localStorage.setItem('username', username)
  if (role !== undefined) localStorage.setItem('role', role)
}

function clearAuth() {
  TOKEN_KEYS.forEach((k) => localStorage.removeItem(k))
  if (window.location.pathname !== '/login') {
    window.location.href = '/login'
  }
}

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// ── 401 → refresh 重放（单飞去重） ─────────────────────────────
let refreshing: Promise<string | null> | null = null

async function doRefresh(): Promise<string | null> {
  try {
    const res = await refreshToken()
    const token = res.data.token
    persistAuth(token, res.data.username, res.data.role)
    return token
  } catch {
    clearAuth()
    return null
  }
}

function getRefreshedToken(): Promise<string | null> {
  if (refreshing) return refreshing
  refreshing = doRefresh().finally(() => {
    refreshing = null
  })
  return refreshing
}

api.interceptors.response.use(
  (res) => res,
  async (error: AxiosError) => {
    const config = error.config as InternalAxiosRequestConfig & { _retried?: boolean }

    // 401 且未重试过且非 refresh/login 自身
    if (
      error.response?.status === 401 &&
      config &&
      !config._retried &&
      !config.url?.includes('/auth/refresh') &&
      !config.url?.includes('/auth/login')
    ) {
      config._retried = true
      const token = await getRefreshedToken()
      if (token) {
        config.headers.Authorization = `Bearer ${token}`
        return api(config)
      }
      // 刷新失败已跳登录
    }

    // 规范化后端统一错误格式 {code, error, message}
    const data = error.response?.data as { error?: string; message?: string } | undefined
    if (data && typeof data === 'object' && 'error' in data && data.error) {
      const normalized = new Error(data.message || error.message) as Error & ApiError
      normalized.code = data.error
      normalized.message = data.message || error.message
      normalized.status = error.response?.status ?? 0
      return Promise.reject(normalized)
    }

    return Promise.reject(error)
  },
)

export default api
