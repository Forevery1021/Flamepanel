import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import BackupView from '@/views/BackupView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站拆分后的视图级冒烟（BackupView 装配层）。
 *
 * 组件级 spec 各自验了表格、弹窗的契约；三个数据层 spec 验了请求编排——
 * 这一层补的是**接线**，它是本站唯一的非平凡逻辑：
 *
 * - 首屏只拉本地备份（另两族在各自页签首次进入时随工具栏一起可用）；
 * - 三个页签的工具条按钮按页签分流（本地 → 创建备份；账号 → 云端恢复 + 新建账号；
 *   任务 → 新建任务）；
 * - 六个子组件全部接线（三张表 + 三个弹窗）；
 * - **跨表引用**：任务表拿到的是账号缓存的**同一份**数据（名字列才有意义）。
 *
 * 挂载策略同前几站：`shallow` + 页签替身渲染全部插槽。
 */

const { api } = vi.hoisted(() => ({
  api: {
    listBackups: vi.fn(),
    listBackupAccounts: vi.fn(),
    listBackupJobs: vi.fn(),
    listRemoteObjects: vi.fn(),
  },
}))

vi.mock('@/api/backups', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backups')>()
  return { ...actual, listBackups: api.listBackups }
})
vi.mock('@/api/backupAccounts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backupAccounts')>()
  return { ...actual, listBackupAccounts: api.listBackupAccounts, listRemoteObjects: api.listRemoteObjects }
})
vi.mock('@/api/backupJobs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/backupJobs')>()
  return { ...actual, listBackupJobs: api.listBackupJobs }
})
vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const messages = {
  backup: {
    title: '数据备份',
    create: '创建备份',
    tabLocal: '本地备份',
    tabAccounts: '备份账号',
    tabJobs: '计划任务',
    cloudRestore: '云端恢复',
    accountCreate: '新建账号',
    jobCreate: '新建任务',
    filename: '备份文件',
    size: '大小',
    createdAt: '创建时间',
    download: '下载',
    restore: '恢复',
    delete: '删除',
    accountName: '账号名称',
    accountType: '类型',
    accountEnabled: '启用',
    accountTest: '测试连接',
    accountTestStatus: '测试结果',
    accountTestedAt: '测试时间',
    accountEdit: '编辑账号',
    jobName: '任务名称',
    jobScope: '备份范围',
    jobSchedule: 'Cron 表达式',
    jobTarget: '目标账号',
    jobRetention: '保留份数',
    jobEnabled: '启用',
    jobLastStatus: '上次状态',
    jobNextRun: '下次执行',
    jobRun: '立即执行',
    jobEdit: '编辑任务',
    accountConfig: '连接配置（JSON）',
    accountConfigEditHint: '编辑时留空表示不修改连接配置（敏感信息不回显）',
    accountTypeLocal: '本地/挂载盘',
    jobScopeKey: '应用 Key',
    cloudRestoreLoad: '加载远端备份',
    cloudRestoreHint: '仅 db 与 app 数据包可还原',
    cloudRestoreKey: '远端对象',
    testNever: '未测试',
    jobStatusNever: '从未执行',
    scopePanel: '面板数据库',
    scopeApp: '应用目录',
  },
  common: {
    noData: '暂无数据',
    cancel: '取消',
    save: '保存',
    close: '关闭',
    edit: '编辑',
    delete: '删除',
    required: '此项为必填项',
    colActions: '操作',
    success: '操作成功',
    failed: '操作失败',
    confirm: '确定',
  },
}

/**
 * 页签替身：`shallow` 下 Fp* 子组件不渲染插槽，而本用例要断言三个页签的接线。
 * 用**纯对象**写法（非 `defineComponent`）避免 `vue/one-component-per-file` 告警——
 * 与 ContainerListPanel.spec / WebsiteListPanel.spec 同一约定。
 */
const TabsStub = {
  name: 'FpTabs',
  props: { items: { type: Array, default: () => [] }, modelValue: { type: String, default: '' } },
  emits: ['update:modelValue'],
  template:
    '<div class="stub-tabs"><slot name="local" /><slot name="accounts" /><slot name="jobs" /></div>',
}

/** LayoutContent 替身：保留 toolbar 插槽（本用例要断言页签分流的按钮） */
const LayoutStub = {
  name: 'LayoutContent',
  props: { title: { type: String, default: '' }, reload: { type: Boolean, default: false } },
  emits: ['reload'],
  template:
    '<div class="stub-layout" :data-title="title"><div class="stub-toolbar"><slot name="toolbar" /></div><slot /></div>',
}

function mountView() {
  return mount(BackupView, {
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
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { FpInput: false, FpSelect: false, FpButton: false, FpTextarea: false, FpTabs: TabsStub, LayoutContent: LayoutStub },
    },
  })
}

async function settle() {
  await new Promise((r) => setTimeout(r, 30))
}

async function switchTab(w: ReturnType<typeof mountView>, value: string) {
  w.findComponent({ name: 'FpTabs' }).vm.$emit('update:modelValue', value)
  await settle()
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listBackups.mockResolvedValue({ data: [] })
  api.listBackupAccounts.mockResolvedValue({ data: [] })
  api.listBackupJobs.mockResolvedValue({ data: [] })
  api.listRemoteObjects.mockResolvedValue({ data: [] })
})

describe('BackupView 装配层（T2-4 第四站冒烟）', () => {
  it('挂载即拉本地备份列表（默认页签，不惰性）', async () => {
    mountView()
    await settle()
    expect(api.listBackups).toHaveBeenCalled()
  })

  it('三个页签名齐备（本地备份 / 备份账号 / 计划任务）', async () => {
    const w = mountView()
    await settle()
    const tabs = w.findComponent({ name: 'FpTabs' })
    expect((tabs.props('items') as { value: string }[]).map((i) => i.value)).toEqual([
      'local',
      'accounts',
      'jobs',
    ])
  })

  it('六个子组件全部接线（三张表 + 三个弹窗）', async () => {
    const w = mountView()
    await settle()
    for (const name of [
      'LocalBackupPanel',
      'BackupAccountPanel',
      'BackupJobPanel',
      'RestoreCloudDialog',
      'BackupAccountDialog',
      'BackupJobDialog',
    ]) {
      expect(w.findComponent({ name }).exists(), `${name} 未接线`).toBe(true)
    }
  })

  it('【关键】任务表拿到账号缓存的同一份数据（跨表译名的前提）', async () => {
    api.listBackupAccounts.mockResolvedValue({
      data: [
        { id: 7, name: 'oss', account_type: 's3', enabled: true, created_at: '', updated_at: '', last_test_at: null, last_test_status: null },
      ],
    })
    const w = mountView()
    await settle()

    const jobPanel = w.findComponent({ name: 'BackupJobPanel' })
    expect(jobPanel.props('accounts')).toEqual([
      expect.objectContaining({ id: 7, name: 'oss' }),
    ])
  })

  it('三个弹窗拿到的是同一份启用账号候选（下拉单一事实来源）', async () => {
    api.listBackupAccounts.mockResolvedValue({
      data: [
        { id: 7, name: 'oss', account_type: 's3', enabled: true, created_at: '', updated_at: '', last_test_at: null, last_test_status: null },
        { id: 8, name: 'nas', account_type: 'local', enabled: false, created_at: '', updated_at: '', last_test_at: null, last_test_status: null },
      ],
    })
    const w = mountView()
    await settle()

    const expected = [{ label: 'oss (S3)', value: 7 }]
    expect(w.findComponent({ name: 'BackupJobDialog' }).props('accountOptions')).toEqual(expected)
    expect(w.findComponent({ name: 'RestoreCloudDialog' }).props('accountOptions')).toEqual(expected)
  })

  it('工具栏按钮按页签分流：本地 → 创建备份', async () => {
    const w = mountView()
    await settle()
    expect(w.find('.stub-toolbar').text()).toContain('创建备份')
  })

  it('工具栏按钮按页签分流：账号 → 云端恢复 + 新建账号', async () => {
    const w = mountView()
    await settle()
    await switchTab(w, 'accounts')
    const text = w.find('.stub-toolbar').text()
    expect(text).toContain('云端恢复')
    expect(text).toContain('新建账号')
  })

  it('工具栏按钮按页签分流：任务 → 新建任务', async () => {
    const w = mountView()
    await settle()
    await switchTab(w, 'jobs')
    expect(w.find('.stub-toolbar').text()).toContain('新建任务')
  })

  it('【关键】点「云端恢复」：先把选中账号定为启用账号首项，再按该账号拉对象', async () => {
    api.listBackupAccounts.mockResolvedValue({
      data: [
        { id: 7, name: 'oss', account_type: 's3', enabled: true, created_at: '', updated_at: '', last_test_at: null, last_test_status: null },
      ],
    })
    const w = mountView()
    await settle()
    await switchTab(w, 'accounts')

    const toolbarButtons = w.findAll('.stub-toolbar button')
    const restoreBtn = toolbarButtons.find((b) => b.text().includes('云端恢复'))
    expect(restoreBtn, '「云端恢复」按钮未渲染').toBeTruthy()
    await restoreBtn!.trigger('click')
    await settle()

    expect(w.findComponent({ name: 'RestoreCloudDialog' }).props('accountId')).toBe(7)
    expect(api.listRemoteObjects).toHaveBeenCalledWith(7)
  })

  it('没有启用账号时点「云端恢复」：不发 objects/0 请求（本地拦截）', async () => {
    api.listBackupAccounts.mockResolvedValue({ data: [] })
    const w = mountView()
    await settle()
    await switchTab(w, 'accounts')

    const restoreBtn = w
      .findAll('.stub-toolbar button')
      .find((b) => b.text().includes('云端恢复'))
    await restoreBtn!.trigger('click')
    await settle()

    expect(api.listRemoteObjects).not.toHaveBeenCalled()
  })

  it('切换页签不重复拉取本地备份（三族各自独立，互不重取）', async () => {
    const w = mountView()
    await settle()
    const before = api.listBackups.mock.calls.length

    await switchTab(w, 'accounts')
    await switchTab(w, 'jobs')
    await switchTab(w, 'local')
    expect(api.listBackups.mock.calls.length).toBe(before)
  })
})
