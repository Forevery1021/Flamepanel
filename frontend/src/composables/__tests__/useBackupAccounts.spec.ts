import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站的数据层单测（`BackupView` → `useBackupAccounts`）。
 *
 * 三条要点，都是「改错不报错、只会悄悄清空用户配置」那一类：
 * ① **编辑留空 = 不修改连接配置**——空串必须转成 `null`（协议里 null 才是保持原值）；
 * ② 启用/停用**无论成败都刷新**（账号状态可能已被后端改动，回到真值更安全）；
 * ③ 远端对象按账号寻址：切换账号必须清空上一账号的清单。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listBackupAccounts: vi.fn(),
    createBackupAccount: vi.fn(),
    updateBackupAccount: vi.fn(),
    deleteBackupAccount: vi.fn(),
    testBackupAccount: vi.fn(),
    listRemoteObjects: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: {
    last: null as { accept?: () => void; reject?: () => void; message: string } | null,
  },
}))

vi.mock('@/api/backupAccounts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backupAccounts')>()
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

import { useBackupAccounts } from '@/composables/useBackupAccounts'

const messages = {
  backup: {
    accountEnableConfirm: '确定启用账号 {name} 吗？',
    accountDisableConfirm: '确定停用账号 {name} 吗？停用后相关备份任务将失败。',
    accountDeleteConfirm: '确定要删除账号 {name} 吗？关联的计划任务将无法执行。',
    testSuccess: '成功',
    testFailed: '失败',
  },
  common: {
    success: '操作成功',
    failed: '操作失败',
    confirm: '确定',
    delete: '删除',
    cancel: '取消',
  },
}

const accountDto = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'oss',
  account_type: 's3',
  enabled: true,
  created_at: '',
  updated_at: '',
  last_test_at: null,
  last_test_status: null,
  ...over,
})

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useBackupAccounts() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useBackupAccounts> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listBackupAccounts.mockResolvedValue({
    data: [accountDto(), accountDto({ id: 2, name: 'nas', account_type: 'local', enabled: false })],
  })
  api.createBackupAccount.mockResolvedValue({ data: accountDto() })
  api.updateBackupAccount.mockResolvedValue({ data: accountDto() })
  api.deleteBackupAccount.mockResolvedValue({ data: null })
  api.testBackupAccount.mockResolvedValue({ data: accountDto({ last_test_status: 'success' }) })
  api.listRemoteObjects.mockResolvedValue({ data: [{ key: 'panel.db', size: 10 }] })
})

describe('useBackupAccounts · 列表与候选', () => {
  it('挂载即拉账号列表', async () => {
    const ctl = mountCtl()
    await settle()
    expect(api.listBackupAccounts).toHaveBeenCalledTimes(1)
    expect(ctl.accounts.value).toHaveLength(2)
  })

  it('目标账号候选只含启用账号', async () => {
    const ctl = mountCtl()
    await settle()
    expect(ctl.selectOptions.value).toEqual([{ label: 'oss (S3)', value: 1 }])
  })
})

describe('useBackupAccounts · 编辑态与保存', () => {
  it('beginEdit(row)：记录目标 id（提交走更新分支）', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.beginEdit(accountDto({ id: 7 }) as never)

    const ok = await ctl.save({
      name: 'oss',
      account_type: 's3',
      config_json: '{"a":1}',
      enabled: true,
    })
    expect(ok).toBe(true)
    expect(api.updateBackupAccount).toHaveBeenCalledWith(7, {
      name: 'oss',
      account_type: 's3',
      config_json: '{"a":1}',
      enabled: true,
    })
  })

  it('【关键】编辑时配置留空 → 上报 null（空串会被当成「清空配置」）', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.beginEdit(accountDto({ id: 7 }) as never)

    await ctl.save({ name: 'oss', account_type: 's3', config_json: '   ', enabled: true })
    expect(api.updateBackupAccount).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ config_json: null }),
    )
  })

  it('新建（未 beginEdit）：走创建分支且配置原样 trim 上报', async () => {
    const ctl = mountCtl()
    await settle()

    await ctl.save({
      name: 'new',
      account_type: 'sftp',
      config_json: '  {"host":"x"}  ',
      enabled: true,
    })
    expect(api.createBackupAccount).toHaveBeenCalledWith({
      name: 'new',
      account_type: 'sftp',
      config_json: '{"host":"x"}',
      enabled: true,
    })
  })

  it('保存失败：返回 false、toast 错误、提交态复位', async () => {
    api.createBackupAccount.mockRejectedValue(new Error('bad json'))
    const ctl = mountCtl()
    await settle()

    const ok = await ctl.save({ name: 'new', account_type: 's3', config_json: '{}', enabled: true })
    expect(ok).toBe(false)
    expect(ctl.saving.value).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useBackupAccounts · 启用开关', () => {
  it('停用：确认文案提示「相关任务将失败」，确认后才发请求', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.toggle(accountDto({ id: 2, name: 'nas' }) as never, false)
    expect(confirmState.last?.message).toBe('确定停用账号 nas 吗？停用后相关备份任务将失败。')
    expect(api.updateBackupAccount).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.updateBackupAccount).toHaveBeenCalledWith(2, { enabled: false })
  })

  it('【关键】开关失败也刷新（账号真值可能已被后端改动）', async () => {
    api.updateBackupAccount.mockRejectedValue(new Error('boom'))
    const ctl = mountCtl()
    await settle()
    const before = api.listBackupAccounts.mock.calls.length

    ctl.toggle(accountDto() as never, false)
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(api.listBackupAccounts.mock.calls.length).toBeGreaterThan(before)
  })
})

describe('useBackupAccounts · 连通性测试', () => {
  it('响应体 last_test_status=success → 成功提示并刷新', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackupAccounts.mock.calls.length

    await ctl.test(accountDto() as never)
    expect(api.testBackupAccount).toHaveBeenCalledWith(1)
    expect(toast.success).toHaveBeenCalledWith('成功')
    expect(api.listBackupAccounts.mock.calls.length).toBeGreaterThan(before)
    expect(ctl.testingId.value).toBeNull()
  })

  it('【关键】接口 200 但 last_test_status=failed → 走错误提示（不能报成功）', async () => {
    api.testBackupAccount.mockResolvedValue({ data: accountDto({ last_test_status: 'failed' }) })
    const ctl = mountCtl()
    await settle()

    await ctl.test(accountDto() as never)
    expect(toast.error).toHaveBeenCalledWith('失败')
    expect(toast.success).not.toHaveBeenCalled()
  })

  it('测试态只锁当前行（请求期间 testingId 为该行 id）', async () => {
    let release: () => void = () => {}
    api.testBackupAccount.mockImplementation(
      () => new Promise((resolve) => (release = () => resolve({ data: accountDto() }))),
    )
    const ctl = mountCtl()
    await settle()

    const pending = ctl.test(accountDto({ id: 9 }) as never)
    await settle()
    expect(ctl.testingId.value).toBe(9)
    release()
    await pending
    expect(ctl.testingId.value).toBeNull()
  })
})

describe('useBackupAccounts · 删除', () => {
  it('确认后删除并刷新（文案提示关联任务将无法执行）', async () => {
    const ctl = mountCtl()
    await settle()
    const before = api.listBackupAccounts.mock.calls.length

    ctl.remove(accountDto({ name: 'oss' }) as never)
    expect(confirmState.last?.message).toBe('确定要删除账号 oss 吗？关联的计划任务将无法执行。')
    confirmState.last?.accept?.()
    await settle()

    expect(api.deleteBackupAccount).toHaveBeenCalledWith(1)
    expect(api.listBackupAccounts.mock.calls.length).toBeGreaterThan(before)
  })
})

describe('useBackupAccounts · 远端对象', () => {
  it('未选账号：loadObjects 不发请求（避免 `objects/0`）', async () => {
    const ctl = mountCtl()
    await settle()

    await ctl.loadObjects()
    expect(api.listRemoteObjects).not.toHaveBeenCalled()
  })

  it('选中账号后按 id 拉对象', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.openRestore(3)

    await ctl.loadObjects()
    expect(api.listRemoteObjects).toHaveBeenCalledWith(3)
    expect(ctl.objects.value).toEqual([{ key: 'panel.db', size: 10 }])
  })

  it('openRestore(账号)：指向该账号并清空上一轮清单', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.openRestore(1)
    await ctl.loadObjects()
    expect(ctl.objects.value).toHaveLength(1)

    ctl.openRestore(2)
    expect(ctl.objects.value).toEqual([])
    expect(ctl.restoreAccountId.value).toBe(2)
  })

  it('【已知契约】切换账号**不**自动重取/清空清单（原实现语义，须再点「加载远端备份」）', async () => {
    const ctl = mountCtl()
    await settle()
    ctl.openRestore(1)
    await ctl.loadObjects()
    expect(ctl.objects.value).toHaveLength(1)

    // 直接改账号（等价于用户在弹窗里换了下拉项）：清单保持不动、不自动重取。
    // 这正是「对着 A 的清单点 B 的还原」缺口的来源——本用例把当前契约写死，
    // 避免将来「顺手清空/顺手重取」时无人察觉（见 PR「后续建议」）。
    ctl.restoreAccountId.value = 2
    await settle()
    expect(ctl.objects.value).toHaveLength(1)
    expect(api.listRemoteObjects).toHaveBeenCalledTimes(1)
  })

  it('对象加载失败：toast 错误且 loading 复位', async () => {
    api.listRemoteObjects.mockRejectedValue(new Error('sftp timeout'))
    const ctl = mountCtl()
    await settle()
    ctl.openRestore(1)

    await ctl.loadObjects()
    expect(toast.error).toHaveBeenCalled()
    expect(ctl.objectsLoading.value).toBe(false)
  })
})
