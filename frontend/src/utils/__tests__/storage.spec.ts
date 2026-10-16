import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { STORAGE_KEYS, writeStorage, rawStringSerializer } from '@/utils/storage'
import { useStorage } from '@vueuse/core'
import { useAuthStore } from '@/stores/auth'

/**
 * `writeStorage`（T0-2 存储单源桥接）回归保护。
 *
 * 背景：`useStorage` 只监听浏览器原生 `storage` 事件，而该事件**只在其它文档**触发——
 * 同文档直写 `localStorage` 不会通知任何订阅者，于是出现「存储已是新值、订阅者仍是
 * 旧值」的分叉。`api/client.ts`（401 刷新回写）无法 import store（会被 store 反向依赖
 * 成环），只能直写存储，因此需要 `writeStorage` 显式广播。
 */

// 隔离网络：这些模块在本 spec 中不应发起真实请求
vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  logout: vi.fn().mockResolvedValue({ data: { message: 'ok' } }),
  refreshToken: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

describe('writeStorage（直写 + 同文档广播）', () => {
  beforeEach(() => {
    localStorage.clear()
  })
  afterEach(() => {
    localStorage.clear()
  })

  it('写入 localStorage', () => {
    writeStorage(STORAGE_KEYS.role, 'admin')
    expect(localStorage.getItem(STORAGE_KEYS.role)).toBe('admin')
  })

  it('传 null 移除键', () => {
    writeStorage(STORAGE_KEYS.role, 'admin')
    writeStorage(STORAGE_KEYS.role, null)
    expect(localStorage.getItem(STORAGE_KEYS.role)).toBeNull()
  })

  it('值未变化时不重复广播（避免无谓的订阅者重算）', () => {
    writeStorage(STORAGE_KEYS.role, 'admin')
    const listener = vi.fn()
    window.addEventListener('storage', listener)
    writeStorage(STORAGE_KEYS.role, 'admin')
    expect(listener).not.toHaveBeenCalled()
    window.removeEventListener('storage', listener)
  })

  it('【关键回归】广播后 useStorage 订阅者（auth store）即时同步', async () => {
    setActivePinia(createPinia())
    const auth = useAuthStore()
    auth.role = 'viewer'
    await nextTick()
    expect(auth.role).toBe('viewer')

    // 模拟 401 刷新链路：client.ts 拿到新角色后直写存储（不能 import store）
    writeStorage(STORAGE_KEYS.role, 'admin')
    await nextTick()

    // 单一来源已同步——v-permission 等订阅方据此重算
    expect(auth.role).toBe('admin')
    expect(localStorage.getItem(STORAGE_KEYS.role)).toBe('admin')
  })

  it('【关键回归】登出清空经广播同步到 store（避免残留旧角色）', async () => {
    setActivePinia(createPinia())
    const auth = useAuthStore()
    auth.role = 'admin'
    await nextTick()

    for (const key of [STORAGE_KEYS.token, STORAGE_KEYS.username, STORAGE_KEYS.role]) {
      writeStorage(key, null)
    }
    await nextTick()

    expect(auth.role).toBe('')
    expect(auth.token).toBe('')
  })

  it('与 useStorage 原始字符串序列化器格式兼容（不加引号）', async () => {
    const role = useStorage<string>(STORAGE_KEYS.role, '', undefined, {
      serializer: rawStringSerializer<string>(),
    })
    writeStorage(STORAGE_KEYS.role, 'operator')
    await nextTick()
    expect(role.value).toBe('operator')
    // 原始字符串格式（非 JSON 引号包裹），与既有持久化数据一致
    expect(localStorage.getItem(STORAGE_KEYS.role)).toBe('operator')
  })
})
