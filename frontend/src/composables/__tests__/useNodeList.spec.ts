import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站的数据层单测（`NodesView` → `useNodeList`）。
 *
 * 这层逻辑原先是「824 行巨型视图内的内联函数」——既在 `src/views/**`（覆盖率
 * 排除区），又没人能脱离整页挂载去验证它。拆出后本 spec 直接看住四件事：
 *
 * ① 列表查询与分页（10s 轮询、保留上一页）；
 * ② **两段式状态覆盖**：列表加载完成后逐节点查 /status + /metrics，且
 *    **写回查询缓存**（视图只有这一个数据源）；
 * ③ **单节点失败不阻断其余**（一台 Agent 掉线不该让整表指标停在旧值）；
 * ④ 页面从后台切回时立即补刷（不干等下一个 10s）。
 */

const { api } = vi.hoisted(() => ({
  api: {
    listNodes: vi.fn(),
    nodeStatus: vi.fn(),
    nodeMetrics: vi.fn(),
  },
}))

vi.mock('@/api/nodes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/nodes')>()
  return { ...actual, ...api }
})

import { useNodeList } from '@/composables/useNodeList'

const row = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'n1',
  hostname: 'h1',
  ip_address: '10.0.0.1',
  status: 'online',
  created_at: '',
  metrics_json: null as string | null,
  ...over,
})

/** 已挂载的宿主（用例间互相隔离：可见性监听是 document 级的，不卸载会串台） */
const mounted: { unmount: () => void }[] = []

/** 挂载宿主：数据层依赖 useApiQuery（需 VueQueryPlugin）与 onMounted/onUnmounted */
function mountList() {
  const Host = defineComponent({
    setup() {
      return { list: useNodeList() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: {
      plugins: [
        createTestI18n({}),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
    },
  })
  mounted.push(w)
  return { w, ctl: (w.vm as unknown as { list: ReturnType<typeof useNodeList> }).list }
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await nextTick()
    await new Promise((r) => setTimeout(r, 10))
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listNodes.mockResolvedValue({ data: { data: [row()], total: 1 } })
  api.nodeStatus.mockResolvedValue({ data: { id: 1, status: 'online' } })
  api.nodeMetrics.mockResolvedValue({
    data: { cpu_usage: 10, memory_usage_percent: 20, disk_usage_percent: 30, load_one: 0.1 },
  })
})

afterEach(() => {
  // 可见性监听挂在 document 上，用例若不卸载会互相串台（上一支用例的节点也会被唤醒）
  for (const w of mounted.splice(0)) w.unmount()
  Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true })
})

describe('useNodeList · 列表', () => {
  it('挂载即拉第一页（20 条/页）', async () => {
    const { ctl } = mountList()
    await settle()
    expect(api.listNodes).toHaveBeenCalledWith(1, 20)
    expect(ctl.nodes.value).toHaveLength(1)
  })

  it('分页参数随 onFirst 变化（first 是偏移量，页号 = first/20 + 1）', async () => {
    api.listNodes.mockResolvedValue({
      data: { data: [row()], total: 100 },
    })
    const { ctl } = mountList()
    await settle()
    expect(ctl.total.value).toBe(100)

    ctl.onFirst(40) // 第 3 页
    await settle()
    expect(api.listNodes).toHaveBeenLastCalledWith(3, 20)
    expect(ctl.first.value).toBe(40)
  })

  it('【关键】列表行的 online 初值恒为 false（真实在线状态只由 /status 判定）', async () => {
    api.nodeStatus.mockReturnValue(new Promise(() => {}))
    api.nodeMetrics.mockReturnValue(new Promise(() => {}))
    const { ctl } = mountList()
    await settle(2)
    expect(ctl.nodes.value[0].online).toBe(false)
  })
})

describe('useNodeList · 两段式状态覆盖', () => {
  it('【关键】列表加载完成后逐节点查状态与指标，并把结果写回缓存（视图只有一个数据源）', async () => {
    api.listNodes.mockResolvedValue({
      data: {
        data: [
          row({ id: 1, metrics_json: '{"cpu_usage":5}' }),
          row({ id: 2, name: 'n2' }),
        ],
        total: 2,
      },
    })
    api.nodeStatus.mockImplementation((id: number) =>
      Promise.resolve({ data: { id, status: id === 1 ? 'online' : 'offline' } }),
    )
    api.nodeMetrics.mockImplementation((id: number) =>
      Promise.resolve({
        data: { cpu_usage: id === 1 ? 11 : 22, memory_usage_percent: 1, disk_usage_percent: 1 },
      }),
    )

    const { ctl } = mountList()
    await settle()

    expect(api.nodeStatus).toHaveBeenCalledWith(1)
    expect(api.nodeStatus).toHaveBeenCalledWith(2)
    expect(api.nodeMetrics).toHaveBeenCalledWith(1)
    expect(ctl.nodes.value.map((n) => n.online)).toEqual([true, false])
    expect(ctl.nodes.value.map((n) => n.metrics?.cpu_usage)).toEqual([11, 22])
  })

  it('【关键】/status 的 edge 概况写回行（缺省为 null，查询失败也清空）', async () => {
    api.listNodes.mockResolvedValue({
      data: { data: [row({ id: 1 }), row({ id: 2, name: 'n2' }), row({ id: 3, name: 'n3' })], total: 3 },
    })
    const edge = { connected: true, terminal: true, version: '1.3.0', connected_at: null }
    api.nodeStatus.mockImplementation((id: number) =>
      id === 3
        ? Promise.reject(new Error('down'))
        : Promise.resolve({ data: { id, status: 'online', ...(id === 1 ? { edge } : {}) } }),
    )
    api.nodeMetrics.mockResolvedValue({ data: {} })
    const { ctl } = mountList()
    await settle()
    expect(ctl.nodes.value.map((n) => n.edge)).toEqual([edge, null, null])
  })

  it('【关键】单节点查询失败不阻断其余：该节点 offline，其余照常覆盖', async () => {
    api.listNodes.mockResolvedValue({
      data: { data: [row({ id: 1 }), row({ id: 2, name: 'n2' })], total: 2 },
    })
    api.nodeStatus.mockImplementation((id: number) =>
      id === 1
        ? Promise.reject(new Error('Agent 不可达'))
        : Promise.resolve({ data: { id, status: 'online' } }),
    )

    const { ctl } = mountList()
    await settle()

    expect(ctl.nodes.value.map((n) => n.online)).toEqual([false, true])
  })

  it('【关键】状态查询失败保留上一次的指标（掉线前的读数仍有诊断价值）', async () => {
    const { ctl } = mountList()
    await settle()
    expect(ctl.nodes.value[0].metrics?.cpu_usage).toBe(10)

    // 第二次覆盖：状态查询失败
    api.nodeStatus.mockRejectedValue(new Error('超时'))
    await ctl.refreshStatus()
    await settle(2)

    expect(ctl.nodes.value[0].online).toBe(false)
    expect(ctl.nodes.value[0].metrics?.cpu_usage).toBe(10)
  })

  it('节点为空时不发状态请求（空列表没有任何可查的目标）', async () => {
    api.listNodes.mockResolvedValue({ data: { data: [], total: 0 } })
    const { ctl } = mountList()
    await settle()
    expect(ctl.nodes.value).toHaveLength(0)
    await ctl.refreshStatus()
    expect(api.nodeStatus).not.toHaveBeenCalled()
  })

  it('【关键】翻页后状态覆盖的是**当前页**（写错 key 会把状态写到别的页上）', async () => {
    api.listNodes.mockImplementation((page: number) =>
      Promise.resolve({
        data: { data: [row({ id: page === 1 ? 1 : 2, name: `n${page}` })], total: 100 },
      }),
    )
    const { ctl } = mountList()
    await settle()
    expect(ctl.nodes.value[0].name).toBe('n1')

    ctl.onFirst(20)
    await settle()

    expect(ctl.nodes.value[0].name).toBe('n2')
    expect(api.nodeStatus).toHaveBeenLastCalledWith(2)
  })
})

describe('useNodeList · 页面可见性', () => {
  it('【关键】从后台切回立即补刷状态（不干等下一个 10s 轮询点）', async () => {
    const { ctl } = mountList()
    await settle()
    api.nodeStatus.mockClear()

    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true })
    document.dispatchEvent(new Event('visibilitychange'))
    await settle(2)
    expect(api.nodeStatus).not.toHaveBeenCalled()

    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true })
    document.dispatchEvent(new Event('visibilitychange'))
    await settle(2)

    expect(ctl.nodes.value[0].online).toBe(true)
    expect(api.nodeStatus).toHaveBeenCalled()
  })

  it('卸载即摘除可见性监听（同一处理器，不会有残留闭包）', async () => {
    const addSpy = vi.spyOn(document, 'addEventListener')
    const removeSpy = vi.spyOn(document, 'removeEventListener')

    const { w } = mountList()
    await settle()
    const entry = addSpy.mock.calls.find((c) => c[0] === 'visibilitychange')
    expect(entry, '未注册可见性监听').toBeTruthy()
    const handler = entry![1]

    w.unmount()
    expect(removeSpy).toHaveBeenCalledWith('visibilitychange', handler)

    addSpy.mockRestore()
    removeSpy.mockRestore()
  })
})
