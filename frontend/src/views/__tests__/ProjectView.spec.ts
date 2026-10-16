import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, enableAutoUnmount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createPinia } from 'pinia'
import ProjectView from '@/views/ProjectView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

enableAutoUnmount(afterEach)

/**
 * T2-4 第六站拆分后的视图级冒烟（ProjectView 装配层）。
 *
 * 组件级 spec 各自验了表格、弹窗、抽屉的契约；四个数据层 spec 验了请求编排——
 * 这一层补的是**接线**，它是本站唯一的非平凡逻辑：
 *
 * - 四个数据层全部挂上（数据层多，漏接一个不会有任何报错，只会静默少一块功能）；
 * - 十类行操作的事件名与数据层动作一一对上（事件名写错同样不报错）；
 * - **两个抽屉的开合与数据层 open/reset 配对**（关闭时清目标，换项目不留残影）；
 * - 列表工具栏的搜索与「新建项目」按钮；
 * - 生命周期动作的刷新回调确实指到列表 `fetch`。
 *
 * 挂载策略同前几站：子组件替身渲染全部具名插槽 + 断言 props 与事件。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listProjects: vi.fn(),
    createProject: vi.fn(),
    updateProject: vi.fn(),
    deleteProject: vi.fn(),
    deployProject: vi.fn(),
    startProject: vi.fn(),
    stopProject: vi.fn(),
    restartProject: vi.fn(),
    checkProjectHealth: vi.fn(),
    enableProjectWebhook: vi.fn(),
    getProjectLogs: vi.fn(),
    listProjectLinks: vi.fn(),
    listLinkableDatabases: vi.fn(),
    listRuntimes: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/projects', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/projects')>()
  return { ...actual, ...api }
})
vi.mock('@/api/runtimes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/runtimes')>()
  return { ...actual, listRuntimes: api.listRuntimes }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const messages = {
  project: {
    title: '项目',
    create: '新建项目',
    createTitle: '新建项目',
    editTitle: '编辑项目',
    name: '项目名称',
    kind: '项目类型',
    source: '来源',
    deployType: '部署方式',
    buildType: '构建器',
    port: '端口',
    domain: '域名',
    deploy: '部署',
    start: '启动',
    stop: '停止',
    restart: '重启',
    healthCheck: '健康检查',
    webhookEnable: 'Webhook',
    webhookReset: '重置令牌',
    webhookTitle: 'Webhook 部署令牌',
    webhookOnceHint: '部署令牌仅本次展示，请立即保存。重复开启将轮换令牌。',
    webhookToken: '部署令牌',
    webhookUrl: 'Webhook 地址',
    webhookPushHint: '将 Webhook 地址配置到 GitHub / GitLab / Gitee / Gitea 的 push 事件。',
    logs: '日志',
    logsTitle: '项目日志 · {name}',
    links: '数据库联链',
    linksTitle: '数据库联链 — {name}',
    linksEmpty: '尚未关联任何数据库实例',
    linksHint: '关联后，标准连接串将在项目启动与部署时自动注入进程环境变量。',
    linkInstance: '数据库实例',
    linkEnvName: '环境变量名（留空按类型默认）',
    linkCreate: '关联',
    linkUnlink: '解除关联',
    linkSuccess: '关联成功，启动项目后生效',
    namePlaceholder: '字母、数字与 -/_，1–64 字符',
    sourceUrl: 'Git 仓库地址',
    sourceUrlPlaceholder: 'https://...',
    runtime: '绑定运行时',
    runCommand: '启动命令',
    runCommandPlaceholder: '如 npm run start',
    deployStrategy: '切换策略',
    healthCheckPath: '健康检查路径',
    healthCheckPathPlaceholder: '如 /health',
    createSuccess: '项目创建成功',
    updateSuccess: '项目更新成功',
    kinds: { php: 'PHP 站点', node: 'Node.js', static: '静态站点', jar: 'Java JAR' },
    sources: { local_path: '本地目录', git: 'Git 仓库' },
    statuses: { building: '构建中', running: '运行中', stopped: '已停止', error: '异常' },
    deployTypes: { manual: '手动部署', webhook: 'Webhook' },
    buildTypes: { dockerfile: '容器构建', buildpacks: 'Buildpacks', runtime: '运行时' },
    strategies: { restart: '直接重启', rolling: '零停机切换（rolling）' },
  },
  common: {
    noData: '暂无数据',
    status: '状态',
    colActions: '操作',
    edit: '编辑',
    delete: '删除',
    cancel: '取消',
    save: '保存',
    close: '关闭',
    copy: '复制',
    retry: '重试',
    success: '操作成功',
    failed: '操作失败',
  },
}

/** LayoutContent 替身：保留 toolbar 插槽（断言搜索与新建按钮） */
const LayoutStub = {
  name: 'LayoutContent',
  props: { title: { type: String, default: '' }, reload: { type: Boolean, default: false } },
  emits: ['reload'],
  template: '<div class="stub-layout" :data-title="title"><slot name="toolbar" /><slot /></div>',
}

function mountView() {
  return mount(ProjectView, {
    global: {
      plugins: [
        createPinia(),
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: {
        LayoutContent: LayoutStub,
        FpInput: false,
        FpButton: false,
        PageToolbar: false,
      },
    },
  })
}

async function settle() {
  await new Promise((r) => setTimeout(r, 30))
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'blog',
  kind: 'node',
  source: 'git',
  source_url: 'https://example.com/blog.git',
  runtime_id: null,
  run_command: 'npm start',
  port: 3000,
  domain: 'blog.example.com',
  health_check_path: '/health',
  deploy_strategy: 'rolling',
  status: 'running',
  deploy_type: 'manual',
  build_type: 'runtime',
  deploy_path: '/opt/x',
  has_procfile: false,
  has_webhook_token: false,
  auto_deploy: false,
  created_at: '',
  updated_at: '',
  ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  api.listProjects.mockResolvedValue({ data: [row()] })
  api.listRuntimes.mockResolvedValue({ data: [] })
  api.getProjectLogs.mockResolvedValue({ data: [{ timestamp: null, message: 'hello' }] })
  api.listProjectLinks.mockResolvedValue({ data: [] })
  api.listLinkableDatabases.mockResolvedValue({ data: [] })
  api.deployProject.mockResolvedValue({ data: row() })
  api.startProject.mockResolvedValue({ data: row() })
  api.stopProject.mockResolvedValue({ data: row() })
  api.restartProject.mockResolvedValue({ data: row() })
  api.checkProjectHealth.mockResolvedValue({ data: { healthy: true, project_id: 3 } })
  api.enableProjectWebhook.mockResolvedValue({
    data: { token: 'tok', webhook_url: '/api/projects/3/webhook?token=tok', project: row() },
  })
})

describe('ProjectView 装配层（T2-4 第六站冒烟）', () => {
  it('每次挂载提供独立 Pinia，且渲染与更新不产生缺失注入警告', async () => {
    const warn = vi.spyOn(console, 'warn')
    try {
      const first = mountView()
      await settle()
      const second = mountView()
      await settle()
      await first.find('.stub-layout input').setValue('blog')
      await settle()

      expect(first.vm.$pinia).toBeDefined()
      expect(second.vm.$pinia).toBeDefined()
      expect(first.vm.$pinia).not.toBe(second.vm.$pinia)
      expect(
        warn.mock.calls.filter(([message]) => String(message).includes('Symbol(pinia)')),
      ).toEqual([])
    } finally {
      warn.mockRestore()
    }
  })

  it('挂载即拉列表与运行时候选（创建弹窗的数据源）', async () => {
    mountView()
    await settle()
    expect(api.listProjects).toHaveBeenCalled()
    expect(api.listRuntimes).toHaveBeenCalled()
  })

  it('【关键】不预拉日志与联链（两个抽屉都是惰性，进页面不该并发打两族请求）', async () => {
    mountView()
    await settle()
    expect(api.getProjectLogs).not.toHaveBeenCalled()
    expect(api.listProjectLinks).not.toHaveBeenCalled()
    expect(api.listLinkableDatabases).not.toHaveBeenCalled()
  })

  it('五个子组件全部接线（表 + 两弹窗 + 两抽屉）', async () => {
    const w = mountView()
    await settle()
    for (const name of [
      'ProjectListPanel',
      'ProjectFormDialog',
      'WebhookDialog',
      'ProjectLogsDrawer',
      'ProjectLinksDrawer',
    ]) {
      expect(w.findComponent({ name }).exists(), `${name} 未接线`).toBe(true)
    }
  })

  it('工具栏：搜索框 + 「新建项目」按钮', async () => {
    const w = mountView()
    await settle()
    const toolbar = w.find('.stub-layout')
    expect(toolbar.text()).toContain('新建项目')
  })

  it('【关键】搜索是把词透传给列表过滤（DOM 里搜出的行确实变少）', async () => {
    api.listProjects.mockResolvedValue({
      data: [row({ id: 1, name: 'blog' }), row({ id: 2, name: 'api', domain: 'api.example.com' })],
    })
    const w = mountView()
    await settle()
    expect(w.findComponent({ name: 'ProjectListPanel' }).props('rows')).toHaveLength(2)

    const input = w.find('.stub-layout input')
    await input.setValue('blog')
    await settle()
    expect(w.findComponent({ name: 'ProjectListPanel' }).props('rows')).toHaveLength(1)
  })

  it('【关键】列表操作事件打到数据层（部署 / 启动 / 停止 / 重启）', async () => {
    const w = mountView()
    await settle()
    const panel = w.findComponent({ name: 'ProjectListPanel' })
    const r = row()

    panel.vm.$emit('deploy', r)
    await settle()
    expect(api.deployProject).toHaveBeenCalledWith(3)

    panel.vm.$emit('start', r)
    await settle()
    expect(api.startProject).toHaveBeenCalledWith(3)

    panel.vm.$emit('stop', r)
    await settle()
    expect(api.stopProject).toHaveBeenCalledWith(3)

    panel.vm.$emit('restart', r)
    await settle()
    expect(api.restartProject).toHaveBeenCalledWith(3)
  })

  it('【关键】健康探测与 Webhook 事件同样打到数据层', async () => {
    const w = mountView()
    await settle()
    const panel = w.findComponent({ name: 'ProjectListPanel' })

    panel.vm.$emit('health', row())
    await settle()
    expect(api.checkProjectHealth).toHaveBeenCalledWith(3)

    panel.vm.$emit('webhook', row())
    await settle()
    expect(api.enableProjectWebhook).toHaveBeenCalledWith(3)
  })

  it('【关键】行操作成功后刷新列表（部署走的是列表的 fetch）', async () => {
    const w = mountView()
    await settle()
    const before = api.listProjects.mock.calls.length

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('deploy', row())
    await settle()
    expect(api.listProjects.mock.calls.length).toBeGreaterThan(before)
  })

  it('【关键】点「日志」：拉该项目的日志并把抽屉打开', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('logs', row())
    await settle()

    expect(api.getProjectLogs).toHaveBeenCalledWith(3)
    const drawer = w.findComponent({ name: 'ProjectLogsDrawer' })
    expect(drawer.props('modelValue')).toBe(true)
    expect(drawer.props('text')).toBe('hello')
  })

  it('【关键】点「联链」：拉联链数据并把抽屉打开', async () => {
    api.listProjectLinks.mockResolvedValue({
      data: [
        {
          id: 11,
          project_id: 3,
          db_instance_id: 5,
          db_instance_name: 'mysql',
          db_type: 'mysql',
          env_name: 'DATABASE_URL',
          port: 3306,
        },
      ],
    })
    const w = mountView()
    await settle()

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('links', row())
    await settle()

    expect(api.listProjectLinks).toHaveBeenCalledWith(3)
    const drawer = w.findComponent({ name: 'ProjectLinksDrawer' })
    expect(drawer.props('modelValue')).toBe(true)
    expect(drawer.props('links')).toHaveLength(1)
  })

  it('【关键】关日志抽屉：清空目标（再打开另一项目不残留上一份日志）', async () => {
    const w = mountView()
    await settle()
    const drawer = w.findComponent({ name: 'ProjectLogsDrawer' })

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('logs', row())
    await settle()
    expect(drawer.props('project')).toMatchObject({ id: 3 })

    drawer.vm.$emit('update:modelValue', false)
    await settle()
    expect(w.findComponent({ name: 'ProjectLogsDrawer' }).props('project')).toBeNull()
  })

  it('【关键】关联链抽屉：清空目标', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('links', row())
    await settle()
    w.findComponent({ name: 'ProjectLinksDrawer' }).vm.$emit('update:modelValue', false)
    await settle()

    expect(w.findComponent({ name: 'ProjectLinksDrawer' }).props('project')).toBeNull()
  })

  it('【关键】打开创建弹窗：拿到的是数据层的运行时候选（同一份，不是空数组）', async () => {
    api.listRuntimes.mockResolvedValue({
      data: [
        {
          id: 7,
          kind: 'node',
          version: '20',
          status: 'installed',
          install_path: '',
          is_default: true,
          created_at: '',
          updated_at: '',
        },
        {
          id: 8,
          kind: 'node',
          version: '18',
          status: 'installed',
          install_path: '',
          is_default: false,
          created_at: '',
          updated_at: '',
        },
      ],
    })
    const w = mountView()
    await settle()
    expect(w.findComponent({ name: 'ProjectFormDialog' }).props('runtimeOptions')).toEqual([
      { label: 'node 20', value: 7 },
    ])
  })

  it('【关键】表单提交成功才关弹窗（失败留在弹窗里让用户改）', async () => {
    document.body.innerHTML = ''
    const w = mountView()
    await settle()
    const dialog = w.findComponent({ name: 'ProjectFormDialog' })
    // 打开弹窗（真组件 + 真 Teleport 到 body，故可见性用 DOM 断言）
    ;(dialog.vm as unknown as { open: () => void }).open()
    await settle()
    expect(document.body.textContent).toContain('新建项目')

    api.createProject.mockRejectedValueOnce(new Error('同名项目已存在'))
    dialog.vm.$emit('create', { name: 'blog' } as never)
    await settle()
    // 失败：弹窗保持打开（用户能改完再提交）
    expect(document.body.textContent).toContain('新建项目')

    api.createProject.mockResolvedValue({ data: row() })
    dialog.vm.$emit('create', { name: 'blog' } as never)
    await settle()
    // 成功：弹窗关闭
    expect(document.body.textContent).not.toContain('新建项目')
  })

  it('【关键】Webhook 令牌窗绑定的是数据层那份令牌（开启后即弹窗）', async () => {
    const w = mountView()
    await settle()

    w.findComponent({ name: 'ProjectListPanel' }).vm.$emit('webhook', row())
    await settle()

    const dialog = w.findComponent({ name: 'WebhookDialog' })
    expect(dialog.props('modelValue')).toBe(true)
    expect(dialog.props('token')).toBe('tok')
  })
})
