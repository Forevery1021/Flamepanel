import api from './client'
import type {
  LoginResponse,
  RecoveryCodesResponse,
  TwoFactorSetup,
  TwoFactorStatusResponse,
} from '@/api/generated'

export function login(username: string, password: string) {
  return api.post<LoginResponse>('/auth/login', { username, password })
}

/** 登录第二步：挑战令牌 + TOTP 验证码或一次性恢复码 */
export function loginTwoFactor(challenge: string, code: string) {
  return api.post<LoginResponse>('/auth/login/2fa', { challenge, code })
}

// ── 两步验证自助管理（Doc/02 §auth 2FA） ──
export function fetchTwoFactorStatus() {
  return api.get<TwoFactorStatusResponse>('/auth/2fa')
}

export function beginTwoFactorSetup(password: string) {
  return api.post<TwoFactorSetup>('/auth/2fa/setup', { password })
}

export function enableTwoFactor(code: string) {
  return api.post<RecoveryCodesResponse>('/auth/2fa/enable', { code })
}

export function disableTwoFactor(password: string, code: string) {
  return api.post<{ message: string }>('/auth/2fa/disable', { password, code })
}

export function regenerateRecoveryCodes(password: string, code: string) {
  return api.post<RecoveryCodesResponse>('/auth/2fa/recovery-codes', { password, code })
}

/** 管理员重置指定用户的两步验证（user:update + admin） */
export function resetUserTwoFactor(userId: number) {
  return api.put<{ reset: boolean }>(`/users/${userId}/2fa/reset`)
}

export function refreshToken() {
  // Y1：refresh token 存于 httpOnly Cookie（XSS 不可读），刷新时由浏览器自动携带
  //（axios 实例已启用 withCredentials）。不再从 localStorage 读取。
  return api.post<LoginResponse>('/auth/refresh')
}

export function logout() {
  // Y1：登出时通知后端吊销 refresh token 并清除 httpOnly Cookie
  return api.post<{ message: string }>('/auth/logout')
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

/**
 * 轮换 JWT 签名密钥（admin）：旧 access token 立即失效。
 * 旧版设置页把随机串写进 `jwt_secret` 设置项——既没有真正轮换，又把「密钥」暴露给只读账号。
 */
export function rotateJwtSecret(secret: string) {
  return api.post('/auth/rotate-secret', { secret })
}
