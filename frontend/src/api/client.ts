import axios, { AxiosError, type AxiosInstance, type InternalAxiosRequestConfig } from 'axios'
import { STORAGE_KEYS, writeStorage } from '@/utils/storage'
import { refreshToken } from './auth'

/** 后端统一错误响应（见 flame-kernel/src/core/error.rs） */
export interface ApiError {
  code: string
  message: string
  status: number
  /** 响应头 `X-Request-Id`（与内核日志中的 request_id 一致，用于排障定位） */
  requestId?: string
}

/** 需要强制修改密码的错误码（后端登录/鉴权返回） */
export const PASSWORD_CHANGE_REQUIRED = 'PASSWORD_CHANGE_REQUIRED'
export const TWO_FACTOR_SETUP_REQUIRED = 'TWO_FACTOR_SETUP_REQUIRED'
export const TWO_FACTOR_SETUP_PATH = '/two-factor-setup'

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
  // Y1：refresh token 存于 httpOnly Cookie，跨请求由浏览器自动携带/写入
  withCredentials: true,
})

const TOKEN_KEYS = [STORAGE_KEYS.token, STORAGE_KEYS.username, STORAGE_KEYS.role] as const

/**
 * 回写凭证（401 刷新链路）。
 *
 * T0-2：经 `writeStorage` 写盘并广播——`auth store` 是前端角色/令牌的**单一来源**
 * （`useStorage` 承载），而本模块被 store 反向依赖（store → api/auth → client），
 * 不能 import store，否则成环。直写 `localStorage` 不会触发同文档的 `storage` 事件，
 * 会让「存储已是新值、store 仍是旧值」分叉，进而使 `v-permission` 等订阅方永远
 * 看不到刷新后的角色。
 */
function persistAuth(token: string, username?: string, role?: string) {
  writeStorage(STORAGE_KEYS.token, token)
  if (username !== undefined) writeStorage(STORAGE_KEYS.username, username)
  if (role !== undefined) writeStorage(STORAGE_KEYS.role, role)
}

function clearAuth() {
  // 兼容清理旧版本可能残留的 refresh_token 键
  localStorage.removeItem(STORAGE_KEYS.refreshToken)
  TOKEN_KEYS.forEach((k) => writeStorage(k, null))
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

// ── 拦截器工厂（T2-1） ─────────────────────────────────────────
// 背景（Doc/23 §2.2 P1-2）：`api` 与 `longApi` 曾各自维护一份**逐行相同**的请求/响应
// 拦截器，双份维护极易「改一处漏一处」（longApi 一份已开始与 api 一份分化）。
// 抽出两个工厂后，两个实例的**唯一差异**只剩「401 重放回哪个实例」，其余行为单源。
//
// 读 token 走 `localStorage` 而非 auth store：本模块被 store 反向依赖
// （store → api/auth → client），import store 会成环（约束同 `persistAuth`）。

/** 请求拦截器：注入 Authorization 头（实例无关）。 */
export function attachAuthInterceptor(instance: AxiosInstance): void {
  instance.interceptors.request.use((config) => {
    const token = localStorage.getItem(STORAGE_KEYS.token)
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  })
}

// ── 401 → refresh 重放（单飞去重） ─────────────────────────────
let refreshing: Promise<string | null> | null = null

async function doRefresh(): Promise<string | null> {
  try {
    const res = await refreshToken()
    const token = res.data.token
    // Y1：refresh token 由后端经 httpOnly Cookie 管理，前端不再持久化
    persistAuth(token, res.data.username, res.data.role)
    return token
  } catch {
    clearAuth()
    return null
  }
}

export function getRefreshedToken(): Promise<string | null> {
  if (refreshing) return refreshing
  refreshing = doRefresh().finally(() => {
    refreshing = null
  })
  return refreshing
}

/**
 * 响应错误拦截器：401 → refresh 单飞重放 + 后端统一错误规范化（实例无关）。
 *
 * `instance` 仅用于**重放**：`api` 重放回 `api`、`longApi` 回 `longApi`——长操作
 * 因此不会在重放时被 15s 默认超时截断（F-33）。
 */
export function attachErrorInterceptor(instance: AxiosInstance): void {
  instance.interceptors.response.use(
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
          return instance(config)
        }
        // 刷新失败已跳登录
      }

      // 规范化后端统一错误格式 {code, error, message}
      const data = error.response?.data as { error?: string; message?: string } | undefined
      if (data && typeof data === 'object' && 'error' in data && data.error) {
        // B3：后端对任意接口返回强制改密错误码时，统一进入改密流程（跳转登录页触发强制改密）
        // Doc/33 TOTP：面板强制两步验证而当前用户未绑定——进入独立绑定页（已在该页则不重复跳转）
        if (
          data.error === TWO_FACTOR_SETUP_REQUIRED &&
          window.location.pathname !== TWO_FACTOR_SETUP_PATH
        ) {
          window.location.href = TWO_FACTOR_SETUP_PATH
        }
        if (data.error === PASSWORD_CHANGE_REQUIRED) {
          clearAuth()
          const force = new URLSearchParams({ force: '1' })
          window.location.href = `/login?${force.toString()}`
        }
        const normalized = new Error(data.message || error.message) as Error & ApiError
        normalized.code = data.error
        normalized.message = data.message || error.message
        normalized.status = error.response?.status ?? 0
        const rid = error.response?.headers?.['x-request-id']
        if (typeof rid === 'string' && rid) normalized.requestId = rid
        return Promise.reject(normalized)
      }

      // 无 HTTP 响应（断网/超时/服务未启动）——保留 AxiosError，isNetworkError 可识别
      if (!error.response) {
        return Promise.reject(error)
      }

      return Promise.reject(error)
    },
  )
}

// ── 实例装配 ─────────────────────────────────────────────────
// 两个实例装配**同一组**拦截器；差异只在构造参数（超时策略）与重放目标（自身）。
attachAuthInterceptor(api)
attachErrorInterceptor(api)

export default api

// ── 长操作专用 axios 实例（F-33） ─────────────────────────────
// 安装应用 / 备份 / 恢复等耗时操作可能超过默认 15s 超时。
// 使用 0 超时（不限制）+ 任务轮询模式，避免请求被中间中断。
const longApi = axios.create({
  baseURL: '/api',
  // F-33: 长操作不设超时（后端经任务轮询返回进度），由调用方决定等待策略
  timeout: 0,
  withCredentials: true,
})

attachAuthInterceptor(longApi)
attachErrorInterceptor(longApi)

export { longApi }
