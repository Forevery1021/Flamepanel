import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import NodesView from '@/views/NodesView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站拆分后的视图级冒烟（NodesView 装配层）。
 *
 * 组件级 spec 各自验了表格与四个弹窗的契约；四个数据层 spec 验了请求编排——
 * 这一层补的是**接线**，它是本站唯一的非平凡编排：
 *
 * - 四个数据层全部挂上（漏接一个不会有任何报错，只会静默少一块功能）；
 * - 四类行操作的事件名与数据层动作一一对上（事件名写错同样不报错）；
 * - 工具栏三个入口分流（注册 / 远程命令 / 批量命令）且批量打开时**全选当前页**；
 * - **列表加载完成后自动覆盖在线状态**（两段式数据流的第二段由装配层触发）。
 *
 * 挂载策略同前几站：子组件替身渲染全部插槽 + 断言 props 与事件。
 */

const { api, toast, confirm } = vi.hoisted(() => ({
  api: {
    listNodes: vi.fn(),
    createNode: vi.fn(),
    updateNode: vi.fn(),
    deleteNode: vi.fn(),
    nodeStatus: vi.fn(),
    nodeMetrics: vi.fn(),
    remoteExecute: vi.fn(),
    batchExecute: vi.fn(),
    remoteListFiles: vi.fn(),
    remoteDownloadFile: vi.fn(),
    remoteUploadFile: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirm: { accept: null as null | (() => Promise<void> | void), action: vi.fn() },
}))

const routerPush = vi.hoisted(() => vi.fn())
vi.mock('vue-router', () => ({ useRouter: () => ({ push: routerPush }) }))

vi.mock('@/api/nodes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/nodes')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept: () => Promise<void> | void }) => {
      confirm.accept = opts.accept
      confirm.action(opts)
    },
  }),
}))

const messages = {
  node: {
    title: '服务器节点',
    register: '注册节点',
    editNode: '编辑节点',
    remoteExec: '远程命令',
    remoteFiles: '远程文件',
    batchExec: '批量命令',
    name: '名称',
    hostname: '主机名',
    ip: 'IP 地址',
    status: '状态',
    id: 'ID',
    metrics: '实时指标',
    lastHeartbeat: '最近心跳',
    liveHint: '节点状态每 10 秒自动刷新（基于 Agent 心跳）',
    command: '命令',
    commandPlaceholder: '输入命令',
    commandRequired: '请输入命令',
    nodeNotSelected: '请至少选择一个节点',
    agentUnreachable: 'Agent 不可达',
    nameRequired: '请输入节点名称',
    hostnameRequired: '请输入主机名',
    ipRequired: '请输入 IP 地址',
    deleteConfirm: '确定要删除节点 {name} 吗？',
    nodeFiles: '节点文件',
    up: '上级目录',
    go: '进入',
    refresh: '刷新',
    download: '下载',
    upload: '上传',
    uploadFile: '上传文件',
    pathPlaceholder: '路径',
    run: '执行',
    runBatch: '批量执行',
    exitCode: '退出码',
    duration: '耗时',
    selectNodesHint: '勾选需要执行命令的节点',
  },
  dashboard: { online: '在线', offline: '离线' },
  common: {
    noData: '暂无数据',
    operation: '操作',
    edit: '编辑',
    delete: '删除',
    confirm: '确认',
    cancel: '取消',
    close: '关闭',
    success: '操作成功',
    failed: '操作失败',
  },
}

/** LayoutContent 替身：保留 toolbar 插槽（断言三个入口按钮） */
const LayoutStub = {
  name: 'LayoutContent',
  props: { title: { type: String, default: '' }, reload: { type: Boolean, default: false } },
  emits: ['reload'],
  template: '<div class="stub-layout" :data-title="title"><slot name="toolbar" /><slot /></div>',
}

const mounted: { unmount: () => void }[] = []

function mountView() {
  const w = mount(NodesView, {
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
      stubs: { LayoutContent: LayoutStub, FpInput: false, FpButton: false, FpSelect: false },
    },
  })
  mounted.push(w)
  return w
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await new Promise((r) => setTimeout(r, 10))
  }
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'db-1',
  hostname: 'db-1.internal',
  ip_address: '10.0.0.1',
  status: 'online',
  created_at: '',
  metrics_json: null as string | null,
  ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  confirm.accept = null
  api.listNodes.mockResolvedValue({ data: { data: [row()], total: 1 } })
  api.nodeStatus.mockResolvedValue({ data: { id: 1, status: 'online' } })
  api.nodeMetrics.mockResolvedValue({
    data: { cpu_usage: 10, memory_usage_percent: 20, disk_usage_percent: 30 },
  })
  api.createNode.mockResolvedValue({ data: 1 })
  api.updateNode.mockResolvedValue({ data: {} })
  api.deleteNode.mockResolvedValue({ data: null })
  api.remoteExecute.mockResolvedValue({ data: { output: 'ok', exit_code: 0, duration_ms: 1 } })
  api.batchExecute.mockResolvedValue({ data: { items: [] } })
  api.remoteListFiles.mockResolvedValue({ data: [] })
})

afterEach(() => {
  for (const w of mounted.splice(0)) w.unmount()
  // 弹窗 Teleport 到 body：卸载后清干净，避免下一支用例读到上一支的 DOM
  document.body.innerHTML = ''
})

describe('NodesView 装配层（T2-4 第七站冒烟）', () => {
  it('挂载即拉列表', async () => {
    mountView()
    await settle()
    expect(api.listNodes).toHaveBeenCalled()
  })

  it('五个子组件全部接线（表 + 四个弹窗）', async () => {
    const w = mountView()
    await settle()
    for (const name of [
      'NodeListPanel',
      'NodeFormDialog',
      'NodeExecDialog',
      'NodeBatchExecDialog',
      'NodeFilesDialog',
    ]) {
      expect(w.findComponent({ name }).exists(), `${name} 未接线`).toBe(true)
    }
  })

  it('工具栏三个入口齐备（注册 / 远程命令 / 批量命令）', async () => {
    const w = mountView()
    await settle()
    const text = w.find('.stub-layout').text()
    expect(text).toContain('注册节点')
    expect(text).toContain('远程命令')
    expect(text).toContain('批量命令')
  })

  it('【关键】列表加载完成后自动覆盖在线状态（两段式数据流的第二段）', async () => {
    mountView()
    await settle()
    expect(api.nodeStatus).toHaveBeenCalledWith(1)
    expect(api.nodeMetrics).toHaveBeenCalledWith(1)
  })

  it('【关键】列表把富化行交给表格（走的是数据层那一份）', async () => {
    api.nodeStatus.mockResolvedValue({ data: { id: 1, status: 'offline' } })
    const w = mountView()
    await settle()

    const panel = w.findComponent({ name: 'NodeListPanel' })
    const rows = panel.props('rows') as { id: number; online: boolean }[]
    expect(rows).toHaveLength(1)
    expect(rows[0].online).toBe(false)
  })

  it('【关键】点「编辑」打开表单弹窗并回填该行', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('edit', row())
    await settle()

    const dialog = w.findComponent({ name: 'NodeFormDialog' })
    expect(dialog.exists()).toBe(true)
    expect(document.body.textContent).toContain('编辑节点')
    expect((document.body.querySelector('input') as HTMLInputElement).value).toBe('db-1')
  })

  it('【关键】点「远程命令」打开弹窗并带上该节点名', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('exec', row())
    await settle()

    const dialog = w.findComponent({ name: 'NodeExecDialog' })
    expect(dialog.props('modelValue')).toBe(true)
    expect(dialog.props('nodeName')).toBe('db-1')
  })

  it('【关键】点「远程文件」打开弹窗、绑定节点并立刻列根目录', async () => {
    const w = mountView()
    await settle()
    api.remoteListFiles.mockClear()

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('files', row())
    await settle()

    const dialog = w.findComponent({ name: 'NodeFilesDialog' })
    expect(dialog.props('modelValue')).toBe(true)
    expect(dialog.props('nodeName')).toBe('db-1')
    expect(api.remoteListFiles).toHaveBeenCalledWith(1, '.')
  })

  it('点「监控」跳转仪表盘并带上节点 ID（Doc/30 M-P）', async () => {
    const w = mountView()
    await settle()
    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('monitor', row())
    expect(routerPush).toHaveBeenCalledWith({ path: '/dashboard', query: { node: '1' } })
  })

  it('点「远程终端」跳转节点终端页并带上节点名（Agent Edge）', async () => {
    const w = mountView()
    await settle()
    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('terminal', row())
    expect(routerPush).toHaveBeenCalledWith({
      name: 'NodeTerminal',
      params: { id: '1' },
      query: { name: row().name },
    })
  })

  it('【关键】点「删除」先确认、确认后发请求并刷新列表', async () => {
    const w = mountView()
    await settle()
    const before = api.listNodes.mock.calls.length

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('remove', row())
    await settle()
    expect(api.deleteNode).not.toHaveBeenCalled()

    await confirm.accept?.()
    await settle()

    expect(api.deleteNode).toHaveBeenCalledWith(1)
    expect(api.listNodes.mock.calls.length).toBeGreaterThan(before)
  })

  it('【关键】工具栏「批量命令」打开即全选当前页', async () => {
    api.listNodes.mockResolvedValue({
      data: { data: [row({ id: 1 }), row({ id: 2, name: 'n2' })], total: 2 },
    })
    const w = mountView()
    await settle()

    const toolbar = w.find('.stub-layout')
    const batchBtn = toolbar
      .findAll('button')
      .find((b) => b.text().includes('批量命令'))!
    await batchBtn.trigger('click')
    await settle()

    const dialog = w.findComponent({ name: 'NodeBatchExecDialog' })
    expect(dialog.props('modelValue')).toBe(true)
    expect(dialog.props('selectedIds')).toEqual([1, 2])
  })

  it('【关键】执行命令的事件打到数据层（远程命令 / 批量命令）', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('exec', row())
    await settle()
    const execDialog = w.findComponent({ name: 'NodeExecDialog' })
    execDialog.vm.$emit('update:command', 'uptime')
    await settle()
    execDialog.vm.$emit('run', 'uptime')
    await settle()
    expect(api.remoteExecute).toHaveBeenCalledWith(1, 'uptime', 30)

    const batchDialog = w.findComponent({ name: 'NodeBatchExecDialog' })
    batchDialog.vm.$emit('update:command', 'df -h')
    batchDialog.vm.$emit('update:selectedIds', [1])
    await settle()
    batchDialog.vm.$emit('run')
    await settle()
    expect(api.batchExecute).toHaveBeenCalledWith([1], 'df -h', 30)
  })

  it('【关键】行操作成功后刷新列表（注册走的是列表 fetch）', async () => {
    const w = mountView()
    await settle()
    const before = api.listNodes.mock.calls.length

    api.createNode.mockResolvedValue({ data: 2 })
    w.findComponent({ name: 'NodeFormDialog' }).vm.$emit('create', {
      name: 'n2',
      hostname: 'h2',
      ip_address: '10.0.0.2',
      status: 'online',
    })
    await settle()

    expect(api.createNode).toHaveBeenCalled()
    expect(api.listNodes.mock.calls.length).toBeGreaterThan(before)
  })

  it('【关键】翻页事件透到列表查询（写错 key 会翻页不动或覆盖错页）', async () => {
    api.listNodes.mockResolvedValue({ data: { data: [row()], total: 100 } })
    const w = mountView()
    await settle()

    w.findComponent({ name: 'NodeListPanel' }).vm.$emit('update:first', 20)
    await settle()
    expect(api.listNodes).toHaveBeenLastCalledWith(2, 20)
  })
})
