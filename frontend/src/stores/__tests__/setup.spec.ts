import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useSetupStore } from '@/stores/setup'
import { getSetupStatus, initializeSetup } from '@/api/setup'

/**
 * T1-2 黄金路径①（Setup 向导全流程 → 进入面板）暴露的缺陷回归。
 *
 * **缺陷**：初始化状态被 store 长期缓存（`statusLoaded`），而 `complete()` 写路径
 * 没有同步它。向导期间缓存的是 `in_progress`，admin 步提交完成（后端已 `completed`）
 * 后 `router.push('/dashboard')` 被守卫用**过期缓存**判回 `/setup`——用户提交完初始化
 * 却停留在向导页，且刷新也不能自愈（`/setup` 对所有访问直接放行、不再探测状态），
 * 必须手工改 URL 才能进入面板。浏览器实测（Playwright，真实后端 + 真实产物）复现。
 *
 * 本 spec 用真实 store + 真实守卫判据（`ensureStatus`）锁定修复：写路径完成后
 * 不得再返回 `in_progress`。
 */

vi.mock('@/api/setup', () => ({
  getSetupStatus: vi.fn(),
  initializeSetup: vi.fn(),
}))

const inProgressStatus = {
  status: 'in_progress' as const,
  theme: 'flame',
  language: 'zh-CN',
  docker: null,
  nginx: null,
}
const completedStatus = { ...inProgressStatus, status: 'completed' as const }

describe('T1-2 缺陷回归：向导完成后不得被过期状态缓存拦回 /setup', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.mocked(getSetupStatus).mockReset()
    vi.mocked(initializeSetup).mockReset()
  })

  it('【核心回归】admin 步完成后 ensureStatus（守卫判据）不再返回 in_progress', async () => {
    // 向导期间的首次探测：后端仍是 in_progress
    vi.mocked(getSetupStatus).mockResolvedValueOnce({ data: inProgressStatus } as never)
    const store = useSetupStore()
    await store.ensureStatus()
    expect(store.status?.status).toBe('in_progress')

    // 向导提交完成（后端响应 completed）
    vi.mocked(initializeSetup).mockResolvedValueOnce({
      data: { status: 'completed', message: 'Setup completed' },
    } as never)
    await store.complete({ step: 'admin', admin: { username: 'e2eadmin', password: 'Pw123456' } })

    // 守卫随后的判定：后端真值已 completed（修复前此处读到缓存的 in_progress → 拦回 /setup）
    vi.mocked(getSetupStatus).mockResolvedValue({ data: completedStatus } as never)
    const s = await store.ensureStatus()
    expect(s?.status).toBe('completed')
  })

  it('后端复探失败时仍按已完成的响应判定（离线兜底，不回落向导）', async () => {
    vi.mocked(getSetupStatus).mockResolvedValueOnce({ data: inProgressStatus } as never)
    const store = useSetupStore()
    await store.ensureStatus()

    vi.mocked(initializeSetup).mockResolvedValueOnce({
      data: { status: 'completed', message: 'Setup completed' },
    } as never)
    await store.complete({ step: 'admin', admin: { username: 'e2eadmin', password: 'Pw123456' } })

    vi.mocked(getSetupStatus).mockRejectedValue(new Error('network down'))
    const s = await store.ensureStatus()
    expect(s?.status).toBe('completed')
  })

  it('database 步（仍 in_progress）不作完成态改写', async () => {
    vi.mocked(getSetupStatus).mockResolvedValueOnce({ data: inProgressStatus } as never)
    const store = useSetupStore()
    await store.ensureStatus()

    vi.mocked(initializeSetup).mockResolvedValueOnce({
      data: { status: 'in_progress', message: 'Database settings saved' },
    } as never)
    await store.complete({ step: 'database', database: { db_type: 'sqlite' } })

    vi.mocked(getSetupStatus).mockResolvedValue({ data: inProgressStatus } as never)
    const s = await store.ensureStatus()
    expect(s?.status).toBe('in_progress')
  })
})
