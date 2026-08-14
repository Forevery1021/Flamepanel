import { defineStore } from 'pinia'
import { computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'
import { login as loginApi } from '@/api/auth'
import type { LoginResponse } from '@/api/generated'

export const useAuthStore = defineStore('auth', () => {
  // P6：改用 @vueuse/core useStorage 统一持久化（保持既有原始字符串存储格式）
  const token = useStorage<string>(STORAGE_KEYS.token, '', undefined, {
    serializer: rawStringSerializer<string>(),
  })
  const refreshToken = useStorage<string>(STORAGE_KEYS.refreshToken, '', undefined, {
    serializer: rawStringSerializer<string>(),
  })
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
    refreshToken.value = res.refresh_token
    username.value = res.username
    role.value = res.role
  }

  async function login(usr: string, pass: string) {
    const res = await loginApi(usr, pass)
    save(res.data)
    return res.data
  }
  function logout() {
    token.value = ''
    refreshToken.value = ''
    username.value = ''
    role.value = ''
    // 清空持久化，使登出后本地不残留凭证
    localStorage.removeItem(STORAGE_KEYS.token)
    localStorage.removeItem(STORAGE_KEYS.refreshToken)
    localStorage.removeItem(STORAGE_KEYS.username)
    localStorage.removeItem(STORAGE_KEYS.role)
  }

  return { token, refreshToken, username, role, isLoggedIn, isAdmin, login, logout, save }
})
