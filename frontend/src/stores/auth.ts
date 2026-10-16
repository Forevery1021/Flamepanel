import { defineStore } from 'pinia'
import { computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'
import {
  login as loginApi,
  loginTwoFactor as loginTwoFactorApi,
  fetchMe,
  logout as logoutApi,
} from '@/api/auth'
import type { LoginResponse } from '@/api/generated'

export const useAuthStore = defineStore('auth', () => {
  // P6：改用 @vueuse/core useStorage 统一持久化（保持既有原始字符串存储格式）
  const token = useStorage<string>(STORAGE_KEYS.token, '', undefined, {
    serializer: rawStringSerializer<string>(),
  })
  // Y1：refresh token 存于 httpOnly Cookie，前端不再持久化（XSS 不可读）。
  const username = useStorage<string>(STORAGE_KEYS.username, '', undefined, {
    serializer: rawStringSerializer<string>(),
  })
  const role = useStorage<string>(STORAGE_KEYS.role, '', undefined, {
    serializer: rawStringSerializer<string>(),
  })

  const isLoggedIn = computed(() => !!token.value)
  const isAdmin = computed(() => role.value === 'admin')

  function save(res: LoginResponse) {
    token.value = res.token
    username.value = res.username
    role.value = res.role
  }

  /** 登录第一步；已启用两步验证时返回 `two_factor_required` + `challenge`，此时不落任何凭证 */
  async function login(usr: string, pass: string) {
    const res = await loginApi(usr, pass)
    if (res.data.two_factor_required) return res.data
    return completeLogin(res.data)
  }

  /** 登录第二步：提交验证码 / 恢复码换取会话 */
  async function loginTwoFactor(challenge: string, code: string) {
    const res = await loginTwoFactorApi(challenge, code)
    return completeLogin(res.data)
  }

  async function completeLogin(data: LoginResponse) {
    save(data)
    // R3：登录后从 /auth/me 同步服务端下发的角色，避免信任可被篡改的本地存储值
    try {
      await refreshMe()
    } catch {
      // 同步失败不影响登录主流程
    }
    return data
  }

  /**
   * R3：从 /auth/me 拉取服务端下发的角色并同步到本地（登录/刷新时调用）。
   * 前端权限以服务端下发为准，降低本地篡改 role 导致的越权观感。
   * 真正的前端越权拦截仍依赖后端每个 API 的服务端 RBAC（底线）。
   */
  async function refreshMe() {
    const res = await fetchMe()
    role.value = res.data.role
    username.value = res.data.username
    return res.data
  }
  function logout() {
    // Y1：先通知后端吊销 refresh token 并清除 httpOnly Cookie（失败不阻塞本地登出）
    logoutApi().catch(() => {})
    token.value = ''
    username.value = ''
    role.value = ''
    // 清空持久化，使登出后本地不残留凭证（兼容清理旧版 refresh_token 键）
    localStorage.removeItem(STORAGE_KEYS.token)
    localStorage.removeItem(STORAGE_KEYS.refreshToken)
    localStorage.removeItem(STORAGE_KEYS.username)
    localStorage.removeItem(STORAGE_KEYS.role)
  }

  return {
    token,
    username,
    role,
    isLoggedIn,
    isAdmin,
    login,
    loginTwoFactor,
    logout,
    save,
    refreshMe,
  }
})
