import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站的数据层单测（`BackupView` → `useBackupLocal`）。
 *
 * 三条要点：① 新建/删除**成功才刷新**、失败不刷新（原实现口径）；
 * ② 下载**不刷新**（它不改服务端状态）；③ 恢复成功只 toast，不刷新列表
 * （恢复的是数据库内容，备份文件清单不变）。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listBackups: vi.fn(),
    createBackup: vi.fn(),
    downloadBackup: vi.fn(),
    restoreBackup: vi.fn(),
    deleteBackup: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; reject?: () => void; message: string } | null },
}))

vi.mock('@/api/backups', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backups')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; reject?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useBackupLocal } from '@/composables/useBackupLocal'

const messages = {
  backup: {
    createSuccess: '备份创建成功',
    restoreConfirm: '确定要从备份 {name} 恢复数据库吗？当前数据将被覆盖。',
    restoreSuccess: '恢复成功',
    deleteConfirm: '确定要删除备份 {name} 吗？',
  },
  common: {
    success: '操作成功',
    failed: '操作失败',
    confirm: '确定',
    cancel: '取消',
    delete: '删除',
  },
}

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useBackupLocal() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: {
      plugins: [
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {} },
    },
  })
  return (w.vm as unknown as { ctl: ReturnType<typeof useBackupLocal> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listBackups.mockResolvedValue({
    data: [{ filename: 'panel-2026.db', size: 1024, created_at: '2026-01-01' }],
  })
  api.createBackup.mockResolvedValue({ data: null })
  api.downloadBackup.mockResolvedValue(undefined)
  api.restoreBackup.mockResolvedValue({ data: null })
  api.deleteBackup.mockResolvedValue({ data: null })
})

describe('useBackupLocal · 列表', () => {
  it('挂载即拉列表（本地备份是默认页签，不惰性）', async () => {
    const ctl = mountCtl()
    await settle()
    expect(api.listBackups).toHaveBeenCalledTimes(1)
    expect(ctl.backups.value).toHaveLength(1)
    expect(ctl.loading.value).toBe(false)
  })
})

describe('useBackupLocal · 新建', () => {
  it('成功：toast 创建成功文案并刷新列表', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    const ok = await ctl.create()
    expect(ok).toBe(true)
    expect(toast.success).toHaveBeenCalledWith('备份创建成功')
    expect(api.listBackups.mock.calls.length).toBeGreaterThan(before)
  })

  it('失败：返回 false、toast 错误且不刷新（不掩盖失败首因）', async () => {
    api.createBackup.mockRejectedValue(new Error('disk full'))
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    const ok = await ctl.create()
    expect(ok).toBe(false)
    expect(toast.error).toHaveBeenCalled()
    expect(api.listBackups.mock.calls.length).toBe(before)
    expect(ctl.creating.value).toBe(false)
  })

  it('创建态在请求期间为 true（工具栏按钮转圈）', async () => {
    let release: () => void = () => {}
    api.createBackup.mockImplementation(
      () => new Promise((resolve) => (release = () => resolve({ data: null }))),
    )
    const ctl = mountCtl()
    await settle()

    const pending = ctl.create()
    await settle()
    expect(ctl.creating.value).toBe(true)
    release()
    await pending
    expect(ctl.creating.value).toBe(false)
  })
})

describe('useBackupLocal · 下载', () => {
  it('按文件名下载且**不刷新列表**（下载不改服务端状态）', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    await ctl.download('panel-2026.db')
    expect(api.downloadBackup).toHaveBeenCalledWith('panel-2026.db')
    expect(api.listBackups.mock.calls.length).toBe(before)
  })

  it('下载失败：toast 错误（不抛给调用方，按钮不卡死）', async () => {
    api.downloadBackup.mockRejectedValue(new Error('404'))
    const ctl = mountCtl()
    await settle()

    await expect(ctl.download('gone.db')).resolves.toBeUndefined()
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useBackupLocal · 恢复与删除', () => {
  it('恢复：确认文案带文件名，确认前不发请求；成功后**不刷新列表**', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    ctl.restore('panel-2026.db')
    expect(confirmState.last?.message).toBe('确定要从备份 panel-2026.db 恢复数据库吗？当前数据将被覆盖。')
    expect(api.restoreBackup).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.restoreBackup).toHaveBeenCalledWith('panel-2026.db')
    expect(toast.success).toHaveBeenCalledWith('恢复成功')
    expect(api.listBackups.mock.calls.length).toBe(before)
  })

  it('删除：确认后删文件并刷新列表', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    ctl.remove('panel-2026.db')
    expect(confirmState.last?.message).toBe('确定要删除备份 panel-2026.db 吗？')

    confirmState.last?.accept?.()
    await settle()
    expect(api.deleteBackup).toHaveBeenCalledWith('panel-2026.db')
    expect(api.listBackups.mock.calls.length).toBeGreaterThan(before)
  })

  it('删除失败：toast 错误且不刷新', async () => {
    api.deleteBackup.mockRejectedValue(new Error('permission denied'))
    const ctl = mountCtl()
    await settle()
    const before = api.listBackups.mock.calls.length

    ctl.remove('panel-2026.db')
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(api.listBackups.mock.calls.length).toBe(before)
  })
})
