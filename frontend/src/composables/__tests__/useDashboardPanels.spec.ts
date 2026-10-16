import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'

/**
 * T2-4 第五站三个「独立面板」数据层的单测：
 * 进程 TOP（轮询）/ 今日待办（重拉）/ 常用应用（排序 + 尽力而为）。
 *
 * 三者的共同点是「失败静默」——仪表盘是只读概览，弹错误提示会打断用户在其他
 * 面板上的操作。这层语义拆出前是三个 `catch {}`，拆出后各有明确理由，也有用例锁住。
 */

const { api, polling } = vi.hoisted(() => ({
  api: {
    listTopProcesses: vi.fn(),
    listMemos: vi.fn(),
    updateMemo: vi.fn(),
    listInstalledApps: vi.fn(),
    launchApp: vi.fn(),
  },
  polling: { start: vi.fn(), stop: vi.fn() },
}))

vi.mock('@/api/metrics', () => ({ listTopProcesses: api.listTopProcesses }))
vi.mock('@/api/memos', () => ({ listMemos: api.listMemos, updateMemo: api.updateMemo }))
vi.mock('@/api/appStore', () => ({
  listInstalledApps: api.listInstalledApps,
  launchApp: api.launchApp,
}))
vi.mock('@/composables/usePolling', () => ({
  usePolling: () => ({
    start: polling.start,
    stop: polling.stop,
    pause: vi.fn(),
    resume: vi.fn(),
    isActive: () => true,
  }),
}))

import { useDashboardProcesses, PROCESS_POLL_INTERVAL_MS } from '@/composables/useDashboardProcesses'
import { useDashboardTodos, TODO_LIMIT } from '@/composables/useDashboardTodos'
import { useDashboardApps, COMMON_APP_LIMIT } from '@/composables/useDashboardApps'

const memos = Array.from({ length: 9 }, (_, i) => ({
  id: i + 1,
  content: `待办 ${i + 1}`,
  kind: 'todo',
  done: false,
  created_at: '',
  updated_at: '',
}))

/** 完整 InstalledApp 形状（后端 DTO 字段齐备，避免测试夹具与契约脱节） */
const app = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'A',
  package_key: 'a',
  version: '1.0.0',
  mode: 'container',
  status: 'running',
  install_path: '/opt/a',
  params_json: '{}',
  created_at: '',
  updated_at: '',
  launch_count: 0,
  access_url: null as string | null,
  ...over,
})

const apps = [
  app({ id: 1, name: 'A', launch_count: 3, access_url: 'http://a' }),
  app({ id: 2, name: 'B', launch_count: 9, access_url: 'http://b' }),
  app({ id: 3, name: 'C', launch_count: undefined, access_url: null }),
]

function mountWith<T>(use: () => T): T {
  const Host = defineComponent({
    setup() {
      return { ctl: use() }
    },
    template: '<div/>',
  })
  const w = mount(Host)
  return (w.vm as unknown as { ctl: T }).ctl
}

const flush = () => new Promise((r) => setTimeout(r, 10))

beforeEach(() => {
  vi.clearAllMocks()
  api.listTopProcesses.mockResolvedValue({ data: [{ name: 'nginx', cpu: 1, memory_mb: 2, pid: 3 }] })
  api.listMemos.mockResolvedValue({ data: memos })
  api.updateMemo.mockResolvedValue({ data: null })
  api.listInstalledApps.mockResolvedValue({ data: apps })
  api.launchApp.mockResolvedValue({ data: null })
})

describe('useDashboardProcesses · 轮询', () => {
  it('拉取进程列表', async () => {
    const ctl = mountWith(useDashboardProcesses)
    await ctl.fetch()
    expect(ctl.processes.value).toHaveLength(1)
    expect(api.listTopProcesses).toHaveBeenCalledTimes(1)
  })

  it('轮询间隔 10s（与节点页「每 10 秒刷新」的既有口径一致）', () => {
    expect(PROCESS_POLL_INTERVAL_MS).toBe(10000)
  })

  it('轮询不 immediate——首屏数据由视图显式 fetch，避免开机双请求', async () => {
    const ctl = mountWith(useDashboardProcesses)
    await ctl.fetch()
    // usePolling 被调用时传了 immediate: false（mock 不执行 fn，故只断言 prompt 语义）
    expect(api.listTopProcesses).toHaveBeenCalledTimes(1)
    ctl.startPolling()
    ctl.stopPolling()
    expect(polling.start).toHaveBeenCalled()
    expect(polling.stop).toHaveBeenCalled()
  })

  it('失败回落空列表（不抛错、不保留上次的陈旧进程）', async () => {
    api.listTopProcesses.mockRejectedValue(new Error('boom'))
    const ctl = mountWith(useDashboardProcesses)
    await ctl.fetch()
    expect(ctl.processes.value).toEqual([])
  })
})

describe('useDashboardTodos · 截断与重拉', () => {
  it(`只保留前 ${TODO_LIMIT} 条（仪表盘是「扫一眼」场景）`, async () => {
    const ctl = mountWith(useDashboardTodos)
    await ctl.fetch()
    expect(ctl.todos.value).toHaveLength(TODO_LIMIT)
    expect(ctl.todos.value[0].id).toBe(1)
  })

  it('loading 跟随请求生命周期（骨架屏）', async () => {
    const ctl = mountWith(useDashboardTodos)
    expect(ctl.loading.value).toBe(false)
    const p = ctl.fetch()
    expect(ctl.loading.value).toBe(true)
    await p
    expect(ctl.loading.value).toBe(false)
  })

  it('勾选：写成功后重新拉列表（已完成的条目不应留在「今日待办」）', async () => {
    const ctl = mountWith(useDashboardTodos)
    await ctl.fetch()
    api.listMemos.mockClear()
    await ctl.toggle(memos[0], true)
    expect(api.updateMemo).toHaveBeenCalledWith(1, { done: true })
    expect(api.listMemos).toHaveBeenCalledTimes(1)
  })

  it('取消勾选同样提交 done=false（C-3 语义在数据层仍成立）', async () => {
    const ctl = mountWith(useDashboardTodos)
    await ctl.fetch()
    await ctl.toggle(memos[0], false)
    expect(api.updateMemo).toHaveBeenCalledWith(1, { done: false })
  })

  it('写失败静默（不抛错、不刷新）', async () => {
    api.updateMemo.mockRejectedValue(new Error('boom'))
    const ctl = mountWith(useDashboardTodos)
    await ctl.fetch()
    api.listMemos.mockClear()
    await expect(ctl.toggle(memos[0], true)).resolves.toBeUndefined()
    expect(api.listMemos).not.toHaveBeenCalled()
  })

  it('拉取失败回落空列表（空态而非报错）', async () => {
    api.listMemos.mockRejectedValue(new Error('boom'))
    const ctl = mountWith(useDashboardTodos)
    await ctl.fetch()
    expect(ctl.todos.value).toEqual([])
    expect(ctl.loading.value).toBe(false)
  })
})

describe('useDashboardApps · 排序与启动', () => {
  it(`按 launch_count 降序，取前 ${COMMON_APP_LIMIT}（缺失计 0）`, async () => {
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    expect(ctl.apps.value.map((a) => a.name)).toEqual(['B', 'A', 'C'])
  })

  it('超过上限时截断', async () => {
    api.listInstalledApps.mockResolvedValue({
      data: Array.from({ length: 10 }, (_, i) => app({ id: i, name: `app${i}`, launch_count: i })),
    })
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    expect(ctl.apps.value).toHaveLength(COMMON_APP_LIMIT)
    expect(ctl.apps.value[0].name).toBe('app9')
  })

  it('打开应用：记一次启动并新窗口打开 access_url', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null)
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    ctl.open(apps[1])
    expect(api.launchApp).toHaveBeenCalledWith(2)
    expect(openSpy).toHaveBeenCalledWith('http://b', '_blank', 'noopener')
    openSpy.mockRestore()
  })

  it('计数请求失败不提示（它的用途只是记数，不能把「打开应用」变成一次报错）', async () => {
    api.launchApp.mockRejectedValue(new Error('boom'))
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null)
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    ctl.open(apps[1])
    await flush()
    expect(openSpy).toHaveBeenCalled()
    openSpy.mockRestore()
  })

  it('无 access_url 时只计数、不开窗口（不打开 about:blank）', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null)
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    ctl.open(apps[2])
    expect(api.launchApp).toHaveBeenCalledWith(3)
    expect(openSpy).not.toHaveBeenCalled()
    openSpy.mockRestore()
  })

  it('拉取失败回落空列表', async () => {
    api.listInstalledApps.mockRejectedValue(new Error('boom'))
    const ctl = mountWith(useDashboardApps)
    await ctl.fetch()
    expect(ctl.apps.value).toEqual([])
  })
})
