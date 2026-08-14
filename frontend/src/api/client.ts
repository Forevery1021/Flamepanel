import axios, { AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { STORAGE_KEYS } from '@/utils/storage'
import { refreshToken } from './auth'

/** 后端统一错误响应（见 flame-kernel/src/core/error.rs） */
export interface ApiError {
  code: string
  message: string
  status: number
}

/** 需要强制修改密码的错误码（后端登录/鉴权返回） */
export const PASSWORD_CHANGE_REQUIRED = 'PASSWORD_CHANGE_REQUIRED'

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

const TOKEN_KEYS = [
  STORAGE_KEYS.token,
  STORAGE_KEYS.refreshToken,
  STORAGE_KEYS.username,
  STORAGE_KEYS.role,
] as const

function persistAuth(token: string, refreshToken?: string, username?: string, role?: string) {
  localStorage.setItem(STORAGE_KEYS.token, token)
  if (refreshToken !== undefined) localStorage.setItem(STORAGE_KEYS.refreshToken, refreshToken)
  if (username !== undefined) localStorage.setItem(STORAGE_KEYS.username, username)
  if (role !== undefined) localStorage.setItem(STORAGE_KEYS.role, role)
}

function clearAuth() {
  TOKEN_KEYS.forEach((k) => localStorage.removeItem(k))
  if (window.location.pathname !== '/login') {
    window.location.href = '/login'
  }
}

/** 是否为无网络/超时等非 HTTP 错误（断网、CORS、服务未启动） */
export function isNetworkError(e: unknown): boolean {
  const err = e as { isAxiosError?: boolean; code?: string; response?: unknown } | null
  return Boolean(
    err?.isAxiosError &&
      (!err.response ||
        err.code === 'ECONNABORTED' ||
        err.code === 'ERR_NETWORK' ||
        err.code === 'ERR_CANCELED'),
  )
}

api.interceptors.request.use((config) => {
  const token = localStorage.getItem(STORAGE_KEYS.token)
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
    persistAuth(token, res.data.refresh_token, res.data.username, res.data.role)
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

    // 无 HTTP 响应（断网/超时/服务未启动）——保留 AxiosError，isNetworkError 可识别
    if (!error.response) {
      return Promise.reject(error)
    }

    return Promise.reject(error)
  },
)

export default api
