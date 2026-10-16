import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站的数据层单测（`BackupView` → `useBackupJobs`）。
 *
 * 三条要点：① `scope !== 'app'` 时 `scope_key` 上报 `null`（空串会被当成「名为空的
 * 应用」）；② 立即执行**取响应体的 last_status 判成败**（接口 200 而备份失败是
 * 常见形态，报成功会让用户以为备好了）；③ 云端恢复按 key 分流确认文案，
 * 且成功时透出后端回显的 message（含是否需要重启）。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listBackupJobs: vi.fn(),
    createBackupJob: vi.fn(),
    updateBackupJob: vi.fn(),
    deleteBackupJob: vi.fn(),
    toggleBackupJob: vi.fn(),
    runBackupJob: vi.fn(),
    restoreFromCloud: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; reject?: () => void; message: string } | null },
}))

vi.mock('@/api/backupJobs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backupJobs')>()
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

import { useBackupJobs } from '@/composables/useBackupJobs'

const messages = {
  backup: {
    jobEnableConfirm: '确定启用任务 {name} 吗？',
    jobDisableConfirm: '确定停用任务 {name} 吗？',
    jobRunConfirm: '确定立即执行任务 {name} 吗？',
    jobRunSuccess: '备份任务执行成功',
    jobRunFailed: '备份任务执行失败',
    jobDeleteConfirm: '确定要删除任务 {name} 吗？',
    cloudRestorePanelConfirm: '确定从云端恢复面板数据库 {key} 吗？当前数据将被覆盖（恢复前自动二次备份）。',
    cloudRestoreAppConfirm: '确定从云端还原应用数据 {key} 吗？该应用的现有数据目录将被备份内容整体替换（失败自动回滚）。',
  },
  common: { success: '操作成功', failed: '操作失败', confirm: '确定', delete: '删除', cancel: '取消', restore: '恢复' },
}

const jobDto = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'nightly',
  scope: 'panel',
  scope_key: null,
  schedule: '0 3 * * *',
  target_account_id: 1,
  retention: 7,
  enabled: true,
  last_status: 'never',
  next_run_at: null,
  last_run_at: null,
  last_message: null,
  last_remote_key: null,
  created_at: '',
  updated_at: '',
  ...over,
})

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useBackupJobs() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useBackupJobs> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listBackupJobs.mockResolvedValue({ data: [jobDto()] })
  api.createBackupJob.mockResolvedValue({ data: jobDto() })
  api.updateBackupJob.mockResolvedValue({ data: jobDto() })
  api.deleteBackupJob.mockResolvedValue({ data: null })
  api.toggleBackupJob.mockResolvedValue({ data: jobDto() })
  api.runBackupJob.mockResolvedValue({ data: jobDto({ last_status: 'success' }) })
  api.restoreFromCloud.mockResolvedValue({
    data: { filename: 'panel.db', message: '恢复完成，需重启面板生效', scope: 'panel' },
  })
})

const payload = (over: Record<string, unknown> = {}) => ({
  name: 'nightly',
  scope: 'panel',
  scope_key: '',
  schedule: '0 3 * * *',
  target_account_id: 1,
  retention: 7,
  enabled: true,
  ...over,
})

describe('useBackupJobs · 列表与编辑态', () => {
  it('挂载即拉任务列表', async () => {
    const ctl = mountCtl()
    await settle()
    expect(api.listBackupJobs).toHaveBeenCalledTimes(1)
    expect(ctl.jobs.value).toHaveLength(1)
  })

  it('beginEdit(row)：提交走更新分支', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.beginEdit(jobDto({ id: 5 }) as never)

    await ctl.save(payload())
    expect(api.updateBackupJob).toHaveBeenCalledWith(5, expect.objectContaining({ name: 'nightly' }))
  })
})

describe('useBackupJobs · 保存的载荷组装', () => {
  it('【关键】panel 范围：scope_key 上报 null（空串会被当成名为空的应用）', async () => {
    const ctl = mountCtl()
    await settle()

    await ctl.save(payload({ scope: 'panel', scope_key: '' }))
    expect(api.createBackupJob).toHaveBeenCalledWith(
      expect.objectContaining({ scope: 'panel', scope_key: null }),
    )
  })

  it('app 范围：scope_key 原样上报', async () => {
    const ctl = mountCtl()
    await settle()

    await ctl.save(payload({ scope: 'app', scope_key: 'blog' }))
    expect(api.createBackupJob).toHaveBeenCalledWith(
      expect.objectContaining({ scope: 'app', scope_key: 'blog' }),
    )
  })

  it('失败：返回 false、toast 错误、提交态复位', async () => {
    api.createBackupJob.mockRejectedValue(new Error('cron 非法'))
    const ctl = mountCtl()
    await settle()

    const ok = await ctl.save(payload())
    expect(ok).toBe(false)
    expect(ctl.saving.value).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useBackupJobs · 启用 / 立即执行 / 删除', () => {
  it('停用：确认文案带任务名，确认后才请求', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.toggle(jobDto({ name: 'nightly' }) as never, false)
    expect(confirmState.last?.message).toBe('确定停用任务 nightly 吗？')
    confirmState.last?.accept?.()
    await settle()
    expect(api.toggleBackupJob).toHaveBeenCalledWith(1, false)
  })

  it('【关键】立即执行：last_status=success 才报成功', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.run(jobDto() as never)
    expect(confirmState.last?.message).toBe('确定立即执行任务 nightly 吗？')
    confirmState.last?.accept?.()
    await settle()

    expect(api.runBackupJob).toHaveBeenCalledWith(1)
    expect(toast.success).toHaveBeenCalledWith('备份任务执行成功')
    expect(ctl.runningId.value).toBeNull()
  })

  it('【关键】接口 200 但 last_status=failed → 报执行失败（不能报成功）', async () => {
    api.runBackupJob.mockResolvedValue({ data: jobDto({ last_status: 'failed' }) })
    const ctl = mountCtl()
    await settle()

    ctl.run(jobDto() as never)
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalledWith('备份任务执行失败')
    expect(toast.success).not.toHaveBeenCalled()
  })

  it('执行态只锁当前行（请求期间 runningId 为该行 id）', async () => {
    let release: () => void = () => {}
    api.runBackupJob.mockImplementation(
      () => new Promise((resolve) => (release = () => resolve({ data: jobDto() }))),
    )
    const ctl = mountCtl()
    await settle()

    ctl.run(jobDto({ id: 9 }) as never)
    confirmState.last?.accept?.()
    await settle()
    expect(ctl.runningId.value).toBe(9)
    release()
    await settle()
    expect(ctl.runningId.value).toBeNull()
  })

  it('删除：确认文案带任务名，确认后删除并刷新', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.remove(jobDto() as never)
    expect(confirmState.last?.message).toBe('确定要删除任务 nightly 吗？')
    confirmState.last?.accept?.()
    await settle()
    expect(api.deleteBackupJob).toHaveBeenCalledWith(1)
  })
})

describe('useBackupJobs · 云端恢复', () => {
  it('【关键】面板库 key：确认文案走「覆盖面板库」分支', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.confirmRestoreCloud(1, 'panel-2026.db')
    expect(confirmState.last?.message).toBe(
      '确定从云端恢复面板数据库 panel-2026.db 吗？当前数据将被覆盖（恢复前自动二次备份）。',
    )
    expect(api.restoreFromCloud).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.restoreFromCloud).toHaveBeenCalledWith(1, 'panel-2026.db')
  })

  it('【关键】应用数据 key：确认文案走「整体替换应用目录」分支', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.confirmRestoreCloud(1, 'app/blog/20260914.tar.gz')
    expect(confirmState.last?.message).toBe(
      '确定从云端还原应用数据 app/blog/20260914.tar.gz 吗？该应用的现有数据目录将被备份内容整体替换（失败自动回滚）。',
    )
  })

  it('成功：透出后端回显 message（含是否需重启的下一步提示），并刷新任务表', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackupJobs.mock.calls.length

    ctl.confirmRestoreCloud(1, 'panel-2026.db')
    confirmState.last?.accept?.()
    await settle()

    expect(toast.success).toHaveBeenCalledWith('恢复完成，需重启面板生效')
    expect(api.listBackupJobs.mock.calls.length).toBeGreaterThan(before)
  })

  it('失败：toast 错误且还原态复位（按钮不卡死）', async () => {
    api.restoreFromCloud.mockRejectedValue(new Error('对象不存在'))
    const ctl = mountCtl()
    await settle()

    ctl.confirmRestoreCloud(1, 'app/blog/x.tar.gz')
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(ctl.restoringKey.value).toBeNull()
  })
})
