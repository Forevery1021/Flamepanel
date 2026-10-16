import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import DockerView from '@/views/DockerView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆分后的视图级冒烟（DockerView 装配层）。
 *
 * 组件级 spec 各自验了表格、表单、弹窗的契约；五个数据层 spec 验了请求编排——
 * 这一层补的是**接线与惰性加载编排**，它是本站唯一留在装配层的非平凡逻辑：
 *
 * - 首屏只拉容器列表（其余四个页签 enabled=false，不发请求）；
 * - 首次切到某页签 → 恰好一次请求；切回 → **不重复请求**（原实现的 Y3 语义）；
 * - 五个面板与四个弹窗全部就位（子组件挂载即接线成立）。
 *
 * 挂载策略同 WebsitesView.spec：`shallow` + 页签替身渲染全部插槽。
 */

const { api } = vi.hoisted(() => ({
  api: {
    listContainers: vi.fn(),
    listImages: vi.fn(),
    listNetworks: vi.fn(),
    listVolumes: vi.fn(),
    listComposeProjects: vi.fn(),
    composeConvertRun: vi.fn(),
    containerToCompose: vi.fn(),
  },
}))

vi.mock('@/api/docker', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/docker')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const messages = {
  docker: {
    containers: '容器',
    images: '镜像',
    networks: '网络',
    volumes: '卷',
    compose: 'Compose',
    searchPlaceholder: '按名称/ID/镜像搜索',
    prune: '清理',
    containerLogs: '容器日志',
    containerStats: '容器统计',
    detail: '详情',
    noStats: '暂无统计数据',
    noLogs: '暂无日志',
    name: '名称',
    containerId: '容器 ID',
    image: '镜像',
    status: '状态',
    actions: '操作',
    start: '启动',
    stop: '停止',
    restart: '重启',
    logs: '日志',
    stats: '统计',
    inspect: '查看详情',
    rename: '重命名',
    pause: '暂停',
    unpause: '恢复',
    kill: '强制停止',
    remove: '删除',
    tags: '标签',
    imageSize: '大小',
    driver: '驱动',
    scope: '范围',
    connectedContainers: '已连接容器',
    mountpoint: '挂载点',
    createNetwork: '创建网络',
    createVolume: '创建卷',
    connect: '连接',
    disconnect: '断开',
    projectName: '项目名称',
    yaml: 'Compose YAML',
    deploy: '部署',
    up: '启动',
    down: '停止',
    projects: 'Compose 项目',
    projectStatus: '状态',
    projectConfigFiles: '配置文件',
    refresh: '刷新',
    tailLines: '显示行数',
    imageNamePlaceholder: '例如 nginx:latest',
    pull: '拉取',
    renamePlaceholder: '新容器名称',
    connectContainer: '连接容器',
    container: '容器',
    subnet: '子网',
  },
  common: {
    noData: '暂无数据',
    cancel: '取消',
    confirm: '确定',
    placeholder: '请输入',
    loading: '加载中...',
    failed: '操作失败',
    success: '操作成功',
    required: '此项为必填项',
    confirmAction: '确定执行此操作吗？',
  },
}

/** 页签替身：`shallow` 下 Fp* 子组件不渲染插槽，而本用例要断言五个页签的接线 */
const TabsStub = defineComponent({
  name: 'FpTabs',
  props: { items: { type: Array, default: () => [] }, modelValue: { type: String, default: '' } },
  emits: ['update:modelValue'],
  template:
    '<div class="stub-tabs"><slot name="containers" /><slot name="images" /><slot name="networks" /><slot name="volumes" /><slot name="compose" /></div>',
})

const LayoutStub = defineComponent({
  name: 'LayoutContent',
  template: '<div><slot name="actions" /><slot name="toolbar" /><slot /></div>',
})
/** 导入弹窗替身：暴露与真组件一致的 open / close，便于断言成功后关闭、失败时保留 */
const dialogClose = vi.fn()
const RunImportDialogStub = defineComponent({
  name: 'RunImportDialog',
  emits: ['submit'],
  setup(_, { expose }) {
    expose({ open: vi.fn(), close: dialogClose })
    return {}
  },
  template: '<div />',
})
async function mountView() {
  const w = mount(DockerView, {
    shallow: true,
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
      stubs: {
        FpInput: false,
        FpSelect: false,
        FpButton: false,
        FpTextarea: false,
        FpTabs: TabsStub,
        LayoutContent: LayoutStub,
        RunImportDialog: RunImportDialogStub,
      },
    },
  })
  await new Promise((r) => setTimeout(r, 30))
  return w
}

async function switchTab(w: Awaited<ReturnType<typeof mountView>>, value: string) {
  w.findComponent({ name: 'FpTabs' }).vm.$emit('update:modelValue', value)
  await new Promise((r) => setTimeout(r, 30))
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listContainers.mockResolvedValue({ data: [] })
  api.listImages.mockResolvedValue({ data: [] })
  api.listNetworks.mockResolvedValue({ data: [] })
  api.listVolumes.mockResolvedValue({ data: [] })
  api.listComposeProjects.mockResolvedValue({ data: [] })
})

describe('DockerView 装配层（T2-4 第三站冒烟）', () => {
  it('首屏只拉容器（其余四个页签惰性：不发请求，避免首屏 5 倍请求）', async () => {
    await mountView()
    expect(api.listContainers).toHaveBeenCalledTimes(1)
    expect(api.listImages).not.toHaveBeenCalled()
    expect(api.listNetworks).not.toHaveBeenCalled()
    expect(api.listVolumes).not.toHaveBeenCalled()
    expect(api.listComposeProjects).not.toHaveBeenCalled()
  })

  it('五个页签名齐备（容器 / 镜像 / 网络 / 卷 / Compose）', async () => {
    const w = await mountView()
    const tabs = w.findComponent({ name: 'FpTabs' })
    expect((tabs.props('items') as { value: string; label: string }[]).map((i) => i.value)).toEqual(
      ['containers', 'images', 'networks', 'volumes', 'compose'],
    )
  })

  it('十二个子组件全部接线（五个列表 + 日志弹窗 + 两个 JSON 弹窗 + 五个表单弹窗）', async () => {
    const w = await mountView()
    for (const name of [
      'ContainerListPanel',
      'ImageListPanel',
      'NetworkListPanel',
      'VolumeListPanel',
      'ComposePanel',
      'ContainerLogsDialog',
      'RenameDialog',
      'CreateNetworkDialog',
      'CreateVolumeDialog',
      'ConnectContainerDialog',
      'RunImportDialog',
    ]) {
      expect(w.findComponent({ name }).exists(), `${name} 未接线`).toBe(true)
    }
    // 两个 JSON 弹窗同组件（统计 + 详情），按 header-key 区分
    const jsonDialogs = w.findAllComponents({ name: 'ContainerJsonDialog' })
    expect(jsonDialogs).toHaveLength(2)
    expect(jsonDialogs.map((d) => d.props('headerKey'))).toEqual([
      'docker.containerStats',
      'docker.detail',
    ])
  })

  it('首次切到镜像页签：恰好一次请求', async () => {
    const w = await mountView()
    await switchTab(w, 'images')
    expect(api.listImages).toHaveBeenCalledTimes(1)
  })

  it('切回已加载的页签：**不重复请求**（Y3 惰性语义，loadedTabs 集合生效）', async () => {
    const w = await mountView()
    await switchTab(w, 'images')
    await switchTab(w, 'containers')
    await switchTab(w, 'images')
    expect(api.listImages).toHaveBeenCalledTimes(1)
  })

  it('四个惰性页签各自被首次切换到才加载（网络 / 卷 / Compose 各一次）', async () => {
    const w = await mountView()
    await switchTab(w, 'networks')
    await switchTab(w, 'volumes')
    await switchTab(w, 'compose')

    expect(api.listNetworks).toHaveBeenCalledTimes(1)
    expect(api.listVolumes).toHaveBeenCalledTimes(1)
    expect(api.listComposeProjects).toHaveBeenCalledTimes(1)
  })

  it('容器页签首屏已加载，切回不额外请求（初始 loadedTabs 含 containers）', async () => {
    const w = await mountView()
    await switchTab(w, 'images')
    await switchTab(w, 'containers')
    expect(api.listContainers).toHaveBeenCalledTimes(1)
  })
})

describe('DockerView · docker run 导入与容器收编（M-D）', () => {
  const conversion = {
    project_name: 'web',
    service_name: 'web',
    compose_yaml: 'services:\n  web:\n    image: nginx:alpine\n',
    warnings: ['--rm 在 Compose 中无对应项，已忽略'],
  }

  it('导入成功：回填 Compose 表单 + 展示提示 + 切到 Compose 页签（不自动部署）', async () => {
    dialogClose.mockClear()
    api.composeConvertRun.mockResolvedValue({ data: conversion })
    const w = await mountView()
    w.findComponent({ name: 'RunImportDialog' }).vm.$emit('submit', 'docker run --rm nginx:alpine')
    await new Promise((r) => setTimeout(r, 30))

    expect(api.composeConvertRun).toHaveBeenCalledWith('docker run --rm nginx:alpine')
    const panel = w.findComponent({ name: 'ComposePanel' })
    expect(panel.props('name')).toBe('web')
    expect(panel.props('yaml')).toBe(conversion.compose_yaml)
    expect(panel.props('warnings')).toEqual(conversion.warnings)
    expect(w.findComponent({ name: 'FpTabs' }).props('modelValue')).toBe('compose')
    expect(dialogClose).toHaveBeenCalledTimes(1)
  })

  it('导入失败：表单保持原值（不被清空），弹窗不关闭以便修改', async () => {
    dialogClose.mockClear()
    api.composeConvertRun.mockRejectedValue(new Error('bad'))
    const w = await mountView()
    const panel = w.findComponent({ name: 'ComposePanel' })
    const before = panel.props('yaml')
    w.findComponent({ name: 'RunImportDialog' }).vm.$emit('submit', 'ls')
    await new Promise((r) => setTimeout(r, 30))
    expect(panel.props('yaml')).toBe(before)
    expect(panel.props('warnings')).toEqual([])
    expect(dialogClose).not.toHaveBeenCalled()
  })

  it('容器行「导出为 Compose」：按容器 ID 转换并回填', async () => {
    api.containerToCompose.mockResolvedValue({ data: { ...conversion, project_name: 'legacy' } })
    const w = await mountView()
    w.findComponent({ name: 'ContainerListPanel' }).vm.$emit('export-compose', {
      id: 'abc123',
      name: 'legacy',
    })
    await new Promise((r) => setTimeout(r, 30))
    expect(api.containerToCompose).toHaveBeenCalledWith('abc123')
    expect(w.findComponent({ name: 'ComposePanel' }).props('name')).toBe('legacy')
    expect(w.findComponent({ name: 'FpTabs' }).props('modelValue')).toBe('compose')
  })
})
