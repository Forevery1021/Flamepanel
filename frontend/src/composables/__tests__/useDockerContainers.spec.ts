import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站的数据层单测（`DockerView` → `useDockerContainers`）。
 *
 * 这层逻辑原先是「1107 行巨型视图内的内联函数」，既在 `src/views/**`（覆盖率
 * 排除区），也没有任何单测。拆出后本 spec 看住五件事：
 * ① 列表加载与三字段搜索过滤；② 无确认类行操作（启停/重启/强停）的容错壳；
 * ③ 删除与清理的**二次确认分支**（拒绝 → 不发请求）；④ 暂停/恢复的双向语义；
 * ⑤ 三只只读弹窗的失败容错（各自不同的回落值，是用户看到的「错误长什么样」）。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listContainers: vi.fn(),
    startContainer: vi.fn(),
    stopContainer: vi.fn(),
    restartContainer: vi.fn(),
    removeContainer: vi.fn(),
    renameContainer: vi.fn(),
    pauseContainer: vi.fn(),
    unpauseContainer: vi.fn(),
    killContainer: vi.fn(),
    pruneContainers: vi.fn(),
    containerLogs: vi.fn(),
    containerStats: vi.fn(),
    inspectContainer: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: {
    last: null as { accept?: () => void; reject?: () => void; message: string } | null,
  },
}))

vi.mock('@/api/docker', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/docker')>()
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

import { useDockerContainers } from '@/composables/useDockerContainers'
import type { DockerContainer } from '@/types'

const messages = {
  docker: {
    removeConfirm: '确定要删除 {name} 吗？',
    pruneConfirm: '确定要清理未使用的{what}吗？',
    containers: '容器',
    logs: '日志',
  },
  common: {
    success: '操作成功',
    failed: '操作失败',
    confirmAction: '确定执行此操作吗？',
    loading: '加载中...',
    required: '此项为必填项',
  },
}

const container = (over: Partial<DockerContainer> = {}): DockerContainer => ({
  id: 'aaa111',
  image: 'nginx:latest',
  name: 'web',
  status: 'running',
  node_id: 1,
  created_at: '2026-01-01T00:00:00Z',
  ...over,
})

function makeHost() {
  return defineComponent({
    setup() {
      return { ctl: useDockerContainers() }
    },
    template: '<div/>',
  })
}

function mountCtl() {
  const w = mount(makeHost(), {
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
  const ctl = (w.vm as unknown as { ctl: ReturnType<typeof useDockerContainers> }).ctl
  return { w, ctl }
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listContainers.mockResolvedValue({
    data: [
      container({ id: 'a1', name: 'web', image: 'nginx:latest', status: 'running' }),
      container({ id: 'a2', name: 'db', image: 'postgres:16', status: 'exited' }),
    ],
  })
  for (const fn of [
    'startContainer',
    'stopContainer',
    'restartContainer',
    'removeContainer',
    'renameContainer',
    'pauseContainer',
    'unpauseContainer',
    'killContainer',
    'pruneContainers',
  ] as const) {
    api[fn].mockResolvedValue({ data: null })
  }
  api.containerLogs.mockResolvedValue({ data: 'log line 1\nlog line 2' })
  api.containerStats.mockResolvedValue({ data: { cpu: '1.5%' } })
  api.inspectContainer.mockResolvedValue({ data: { Id: 'a1', Name: '/web' } })
})

describe('useDockerContainers · 列表与搜索', () => {
  it('挂载即拉容器列表（首个页签默认启用），并填充过滤结果', async () => {
    const { ctl } = mountCtl()
    await settle()

    expect(api.listContainers).toHaveBeenCalledTimes(1)
    expect(ctl.containers.value).toHaveLength(2)
    expect(ctl.filtered.value).toHaveLength(2)
    expect(ctl.loading.value).toBe(false)
  })

  it('搜索按名称/ID/镜像三字段过滤（大小写不敏感），清空后恢复全量', async () => {
    const { ctl } = mountCtl()
    await settle()

    ctl.search.value = 'POSTGRES'
    await settle()
    expect(ctl.filtered.value.map((c) => c.id)).toEqual(['a2'])

    ctl.search.value = 'a1'
    await settle()
    expect(ctl.filtered.value.map((c) => c.name)).toEqual(['web'])

    ctl.search.value = ''
    await settle()
    expect(ctl.filtered.value).toHaveLength(2)
  })

  it('无命中的搜索得到空数组（驱动表格空态，而不是展示全量）', async () => {
    const { ctl } = mountCtl()
    await settle()

    ctl.search.value = 'zzz'
    await settle()
    expect(ctl.filtered.value).toEqual([])
  })
})

describe('useDockerContainers · 行操作容错壳', () => {
  it('启动成功：toast 成功 + 重新拉取列表', async () => {
    const { ctl } = mountCtl()
    await settle()
    const before = api.listContainers.mock.calls.length

    const ok = await ctl.start(container({ id: 'a2', status: 'exited' }))
    expect(ok).toBe(true)
    expect(api.startContainer).toHaveBeenCalledWith('a2')
    expect(toast.success).toHaveBeenCalledWith('操作成功')
    expect(api.listContainers.mock.calls.length).toBe(before + 1)
  })

  it('启动失败：toast 错误且**不刷新列表**（失败无状态变化，多打一次请求只会掩盖首因）', async () => {
    api.startContainer.mockRejectedValue(new Error('conflict'))
    const { ctl } = mountCtl()
    await settle()
    const before = api.listContainers.mock.calls.length

    const ok = await ctl.start(container({ id: 'a1' }))
    expect(ok).toBe(false)
    expect(toast.error).toHaveBeenCalled()
    expect(api.listContainers.mock.calls.length).toBe(before)
  })

  it('停止 / 重启 / 强停 各打到自己的端点（同一容错壳、不同 API）', async () => {
    const { ctl } = mountCtl()
    await settle()

    await ctl.stop(container({ id: 's1' }))
    await ctl.restart(container({ id: 'r1' }))
    await ctl.kill(container({ id: 'k1' }))

    expect(api.stopContainer).toHaveBeenCalledWith('s1')
    expect(api.restartContainer).toHaveBeenCalledWith('r1')
    expect(api.killContainer).toHaveBeenCalledWith('k1')
  })

  it('暂停/恢复按当前状态择一（running → pause，paused → unpause）', async () => {
    const { ctl } = mountCtl()
    await settle()

    await ctl.togglePause(container({ id: 'p1', status: 'running' }))
    expect(api.pauseContainer).toHaveBeenCalledWith('p1')
    expect(api.unpauseContainer).not.toHaveBeenCalled()

    await ctl.togglePause(container({ id: 'p2', status: 'paused' }))
    expect(api.unpauseContainer).toHaveBeenCalledWith('p2')
  })
})

describe('useDockerContainers · 二次确认分支', () => {
  it('删除：确认文案带容器展示名，**确认前不发请求**', async () => {
    const { ctl } = mountCtl()
    await settle()

    ctl.remove(container({ id: 'a1', name: 'web' }))
    expect(confirmState.last?.message).toBe('确定要删除 web 吗？')
    expect(api.removeContainer).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.removeContainer).toHaveBeenCalledWith('a1')
  })

  it('删除：容器名为空时文案回落 ID（不出现「确定要删除  吗？」）', async () => {
    const { ctl } = mountCtl()
    await settle()

    ctl.remove(container({ id: 'a9', name: '' }))
    expect(confirmState.last?.message).toBe('确定要删除 a9 吗？')
  })

  it('删除：用户取消（不调 accept）则永不发请求', async () => {
    const { ctl } = mountCtl()
    await settle()

    ctl.remove(container({ id: 'a1' }))
    expect(api.removeContainer).not.toHaveBeenCalled()
  })

  it('清理容器：文案带「容器」目标名，确认后调 prune 并刷新', async () => {
    const { ctl } = mountCtl()
    await settle()
    const before = api.listContainers.mock.calls.length

    ctl.prune()
    expect(confirmState.last?.message).toBe('确定要清理未使用的容器吗？')
    expect(api.pruneContainers).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.pruneContainers).toHaveBeenCalledTimes(1)
    expect(api.listContainers.mock.calls.length).toBe(before + 1)
  })

  it('清理失败：toast 错误且不刷新', async () => {
    api.pruneContainers.mockRejectedValue(new Error('boom'))
    const { ctl } = mountCtl()
    await settle()
    const before = api.listContainers.mock.calls.length

    ctl.prune()
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(api.listContainers.mock.calls.length).toBe(before)
  })
})

describe('useDockerContainers · 重命名', () => {
  it('成功：名称 trim 后上报，刷新列表并返回 true（调用方据此关弹窗）', async () => {
    const { ctl } = mountCtl()
    await settle()

    const ok = await ctl.rename('a1', '  web2  ')
    expect(ok).toBe(true)
    expect(api.renameContainer).toHaveBeenCalledWith('a1', 'web2')
    expect(ctl.renaming.value).toBe(false)
  })

  it('失败：返回 false 且提交态复位（弹窗保持打开让用户修正）', async () => {
    api.renameContainer.mockRejectedValue(new Error('in use'))
    const { ctl } = mountCtl()
    await settle()

    const ok = await ctl.rename('a1', 'web2')
    expect(ok).toBe(false)
    expect(ctl.renaming.value).toBe(false)
  })
})

describe('useDockerContainers · 三个只读弹窗的数据', () => {
  it('日志：按容器与行数拉取，内容原样落库', async () => {
    const { ctl } = mountCtl()
    await settle()

    await ctl.loadLogs('web', 500)
    expect(api.containerLogs).toHaveBeenCalledWith('web', 500)
    expect(ctl.logsContent.value).toBe('log line 1\nlog line 2')
  })

  it('日志失败：回落失败文案（弹窗里能看到原因，而不是空白的 pre）', async () => {
    api.containerLogs.mockRejectedValue(new Error('gone'))
    const { ctl } = mountCtl()
    await settle()

    await ctl.loadLogs('web', 100)
    expect(ctl.logsContent.value).toBe('操作失败')
  })

  it('统计成功：原始对象落库（缩进渲染由展示层负责）', async () => {
    const { ctl } = mountCtl()
    await settle()

    await ctl.loadStats('a1')
    expect(ctl.statsData.value).toEqual({ cpu: '1.5%' })
  })

  it('统计失败：置 null（弹窗显示「暂无统计数据」占位，不留上一次的残留值）', async () => {
    const { ctl } = mountCtl()
    await settle()
    await ctl.loadStats('a1')

    api.containerStats.mockRejectedValue(new Error('boom'))
    await ctl.loadStats('a1')
    expect(ctl.statsData.value).toBeNull()
  })

  it('详情：先落「加载中」占位，成功后写 2 空格缩进的 JSON', async () => {
    const { ctl } = mountCtl()
    await settle()

    const pending = ctl.loadDetail('a1')
    expect(ctl.detailContent.value).toBe('加载中...')
    await pending

    expect(ctl.detailContent.value).toBe(JSON.stringify({ Id: 'a1', Name: '/web' }, null, 2))
  })

  it('详情失败：回落失败文案（不把「加载中」永久留在弹窗上）', async () => {
    api.inspectContainer.mockRejectedValue(new Error('boom'))
    const { ctl } = mountCtl()
    await settle()

    await ctl.loadDetail('a1')
    expect(ctl.detailContent.value).toBe('操作失败')
  })
})
