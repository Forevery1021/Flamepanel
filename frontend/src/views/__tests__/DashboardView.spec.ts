import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { enableAutoUnmount, mount } from '@vue/test-utils'
import { nextTick, ref, type Ref } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import DashboardView from '@/views/DashboardView.vue'
import TrendPanel from '@/components/dashboard/TrendPanel.vue'
import type { TrendRange, TrendSample } from '@/utils/dashboardCharts'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
// 开屏优化后三块图表面板在视图内经 `defineAsyncComponent` 懒加载；这里预热模块缓存，
// 让测试中的 `import()` 在几个微任务内解析（否则首个用例需等 SFC 编译，settle(30) 可能不够）
import '@/components/dashboard/TrendPanel.vue'
import '@/components/dashboard/NetworkPanel.vue'
import '@/components/dashboard/LoadPanel.vue'

/**
 * T2-4 第五站拆分后的视图级冒烟（DashboardView 装配层）。
 *
 * 面板各自的契约（指标卡可点击性、进程列格式化、待办只读、图表重绘时机）已由
 * `components/dashboard/__tests__/*` 与各 composable spec 覆盖。这一层补的是
 * **本站唯一的非平凡编排**：
 *
 * ① 七块面板全部接线（一块漏接就是整屏空白）；
 * ② **三图共享节流器**：WS tick 连续到达时只递增一次 `refreshKey`
 *    （若每张图各自 watch 数据，节流会失效——这是本站最关键的一条设计）；
 * ③ 主题切换（根元素 class/style 变化）分发到三个面板的 `applyTheme`；
 * ④ 首屏并行拉取三族独立数据（进程 / 待办 / 常用应用）并启动进程轮询；
 * ⑤ S-16 时间范围：`live` 走 WS 6 min，`1h/24h/7d` 切到 S-07 历史样本——两张折线图
 *    同源切换、历史到达走同一条节流重绘通道、选择持久化到 localStorage。
 *
 * 挂载策略：三张图表面板打桩（jsdom 无画布），其余面板真实挂载。
 */

const { api, ws, polling, echarts, metricsHistory } = vi.hoisted(() => ({
  api: {
    listTopProcesses: vi.fn(),
    listMemos: vi.fn(),
    updateMemo: vi.fn(),
    listInstalledApps: vi.fn(),
    launchApp: vi.fn(),
    listNodes: vi.fn(),
  },
  ws: {
    options: null as null | { onMessage: (d: unknown) => void; onStatus?: (c: boolean) => void },
    /** 每次 useWebSocket 调用的端点与 close 桩（节点切换断言用） */
    opened: [] as { url: string; close: ReturnType<typeof vi.fn> }[],
  },
  polling: { start: vi.fn(), stop: vi.fn() },
  echarts: {
    mount: vi.fn(),
    setOption: vi.fn(),
    applyOption: vi.fn(),
    instance: vi.fn(() => ({})),
  },
  metricsHistory: {
    range: null as null | { value: string },
    samples: null as null | { value: unknown[] },
    loading: null as null | { value: boolean },
    error: null as null | { value: unknown },
    stepSecs: null as null | { value: number },
  },
}))

const nav = vi.hoisted(() => ({
  route: null as null | { query: Record<string, string> },
  replace: vi.fn(),
}))
vi.mock('vue-router', async () => {
  const { reactive } = await import('vue')
  const route = reactive({ query: {} as Record<string, string> })
  nav.route = route
  return {
    useRouter: () => ({
      push: vi.fn(),
      replace: (to: { query: Record<string, string> }) => {
        nav.replace(to)
        route.query = to.query
      },
    }),
    useRoute: () => route,
  }
})
vi.mock('@/api/nodes', () => ({ listNodes: api.listNodes }))
vi.mock('@/api/metrics', () => ({ listTopProcesses: api.listTopProcesses }))
vi.mock('@/api/memos', () => ({ listMemos: api.listMemos, updateMemo: api.updateMemo }))
vi.mock('@/api/appStore', () => ({
  listInstalledApps: api.listInstalledApps,
  launchApp: api.launchApp,
}))
vi.mock('@/composables/useWebSocket', () => ({
  useWebSocket: (
    url: string,
    options: { onMessage: (d: unknown) => void; onStatus?: (c: boolean) => void },
  ) => {
    ws.options = options
    const close = vi.fn()
    ws.opened.push({ url, close })
    return {
      connect: vi.fn(),
      close,
      pause: vi.fn(),
      resume: vi.fn(),
      send: vi.fn(),
      reconnectNow: vi.fn(),
      isActive: () => true,
    }
  },
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
vi.mock('@/composables/useMetricsHistory', () => ({
  useMetricsHistory: (range: { value: string }) => {
    metricsHistory.range = range
    return {
      samples: metricsHistory.samples,
      loading: metricsHistory.loading,
      error: metricsHistory.error,
      stepSecs: metricsHistory.stepSecs,
      isFetching: { value: false },
      refresh: vi.fn(),
    }
  },
}))
vi.mock('@/composables/useInspection', () => ({
  // S-17 横幅：仪表盘装配测试不关心体检数据，给一份「无高危项」的空结果
  useInspection: () => ({ report: { value: null }, urgent: { value: [] } }),
}))
vi.mock('@/composables/useECharts', () => ({
  useECharts: () => ({
    mount: echarts.mount,
    setOption: echarts.setOption,
    applyOption: echarts.applyOption,
    instance: echarts.instance,
    resize: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    dispose: vi.fn(),
  }),
}))

const messages = {
  dashboard: {
    title: '仪表盘',
    cpu: 'CPU',
    memory: '内存',
    disk: '磁盘',
    load: '系统负载',
    goTo: '查看{module}详情',
    trend: '性能趋势',
    wsConnected: '已连接',
    wsDisconnected: '未连接',
    network: '网络 IO',
    rx: '下行',
    tx: '上行',
    dataPoints: '数据点',
    lastUpdate: '最后更新',
    processTop: '进程 TOP',
    processName: '进程',
    processCpu: 'CPU',
    processMem: '内存',
    processPid: 'PID',
    todayTodos: '今日待办',
    more: '更多',
    noTodos: '今日暂无待办',
    commonApps: '常用应用',
    noCommonApps: '暂无已安装应用',
    waitingData: '等待数据…',
    node: { label: '节点', local: '本机' },
    range: {
      label: '时间范围',
      live: '实时',
      '1h': '1 小时',
      '24h': '24 小时',
      '7d': '7 天',
      avgOf: '{step} 均值',
      loadFailed: '历史加载失败',
    },
  },
  common: { noData: '暂无数据', failed: '操作失败' },
}

const RANGE_KEY = 'fp.dashboard.trendRange'
/** 节点切换器走 useApiQuery（vue-query），每次挂载一个独立 client 防缓存串用例 */
const queryPlugin = () =>
  [
    VueQueryPlugin,
    { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
  ] as [typeof VueQueryPlugin, { queryClient: QueryClient }]
const historyPoint = (ts: number): TrendSample => ({
  timestamp: ts,
  cpu_usage: 1,
  memory_usage_percent: 2,
  disk_usage_percent: 3,
  network_rx_mbps: 0.1,
  network_tx_mbps: 0.2,
})

const snap = (over: Record<string, unknown> = {}) => ({
  timestamp: 1,
  cpu_usage: 10,
  cpu_cores: 4,
  memory_usage_percent: 20,
  memory_total_mb: 1024,
  memory_used_mb: 512,
  disk_usage_percent: 30,
  disk_total_gb: 10,
  disk_used_gb: 3,
  load_one: 1,
  load_five: 1,
  load_fifteen: 0.5,
  network_rx_mbps: 0.5,
  network_tx_mbps: 0.25,
  ...over,
})

function mountView() {
  const pinia = createPinia()
  setActivePinia(pinia)
  return mount(DashboardView, {
    global: {
      plugins: [pinia, queryPlugin(), createTestI18n(messages), openVuePlugin, ...services],
    },
  })
}

const settle = (ms = 30) => new Promise((r) => setTimeout(r, ms))
// 共享的 reactive route 在 beforeEach 中重置；残留的旧视图会响应它而重连，统一自动卸载
enableAutoUnmount(afterEach)
const push = (data: unknown) => ws.options!.onMessage(data)

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  ws.options = null
  ws.opened = []
  nav.route!.query = {}
  api.listNodes.mockResolvedValue({ data: { data: [], total: 0 } })
  localStorage.removeItem(RANGE_KEY)
  metricsHistory.range = null
  metricsHistory.samples = ref<TrendSample[]>([]) as Ref<unknown[]>
  metricsHistory.loading = ref(false)
  metricsHistory.error = ref<unknown>(null)
  metricsHistory.stepSecs = ref(60)
  api.listTopProcesses.mockResolvedValue({ data: [] })
  api.listMemos.mockResolvedValue({ data: [] })
  api.updateMemo.mockResolvedValue({ data: null })
  api.listInstalledApps.mockResolvedValue({ data: [] })
  api.launchApp.mockResolvedValue({ data: null })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('DashboardView · 面板接线', () => {
  it('七块面板全部渲染（指标卡 + 三图表面板 + 三清单面板）', async () => {
    const w = mountView()
    await settle()
    expect(w.findAll('.stat-card')).toHaveLength(4)
    expect(w.find('.panel-trend').exists()).toBe(true)
    expect(w.find('.charts-grid').exists()).toBe(true)
    expect(w.findAll('.info-row')).toHaveLength(2)
    expect(w.find('.todo-list').exists()).toBe(true)
    expect(w.find('.common-apps, .panel-empty').exists()).toBe(true)
  })

  it('首屏并行拉取三族独立数据并启动进程轮询', async () => {
    mountView()
    await settle()
    expect(api.listTopProcesses).toHaveBeenCalledTimes(1)
    expect(api.listMemos).toHaveBeenCalledTimes(1)
    expect(api.listInstalledApps).toHaveBeenCalledTimes(1)
    expect(polling.start).toHaveBeenCalledTimes(1)
  })

  it('指标卡渲染 WS 推送的真实数值（首帧前为骨架，不显示误导性的 0%）', async () => {
    const w = mountView()
    await settle()
    // 开屏优化：未收到首帧 → 数值位是骨架条，而不是 0.0%
    expect(w.find('.stat-skeleton').exists()).toBe(true)
    expect(w.find('.stat-value').text()).toBe('')
    push({ type: 'tick', data: snap({ cpu_usage: 42.5 }) })
    await nextTick()
    expect(w.find('.stat-skeleton').exists()).toBe(false)
    expect(w.find('.stat-value').text()).toBe('42.5%')
  })

  it('init 帧即回填指标卡（不必等下一个 tick）', async () => {
    const w = mountView()
    await settle()
    push({ type: 'init', data: [snap({ cpu_usage: 5 }), snap({ cpu_usage: 66.6 })] })
    await nextTick()
    expect(w.find('.stat-value').text()).toBe('66.6%')
  })

  it('待办勾选上抛到数据层（写请求带 done 布尔）', async () => {
    api.listMemos.mockResolvedValue({
      data: [{ id: 9, content: '待办', kind: 'todo', done: true, created_at: '', updated_at: '' }],
    })
    const w = mountView()
    await settle()
    // 从「已完成」取消勾选：change 载荷必须是 false（C-3 缺陷下恒为 true）
    await w.find('.todo-item input[type="checkbox"]').setValue(false)
    await settle()
    expect(api.updateMemo).toHaveBeenCalledWith(9, { done: false })
  })

  it('viewer 只读：待办不渲染勾选框、常用应用按钮置灰', async () => {
    api.listInstalledApps.mockResolvedValue({
      data: [{ id: 1, name: 'Nginx', launch_count: 2, access_url: 'http://n' }],
    })
    const pinia = createPinia()
    setActivePinia(pinia)
    useAuthStore().role = 'viewer'
    const w = mount(DashboardView, {
      global: {
        plugins: [pinia, queryPlugin(), createTestI18n(messages), openVuePlugin, ...services],
      },
    })
    await settle()
    expect(w.find('.todo-item input[type="checkbox"]').exists()).toBe(false)
    expect(w.find('.common-app').attributes('disabled')).toBeDefined()
  })
})

describe('DashboardView · 三图刷新编排（本站核心）', () => {
  it('连续 WS tick 只触发一次刷新（1s 节流合并）', async () => {
    mountView()
    await settle()
    echarts.setOption.mockClear()
    push({ type: 'tick', data: snap({ timestamp: 1 }) })
    push({ type: 'tick', data: snap({ timestamp: 2 }) })
    push({ type: 'tick', data: snap({ timestamp: 3 }) })
    // 窗口未到期：一次都没刷
    expect(echarts.setOption).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1000)
    await nextTick()
    // 到点后三张图各刷一次（trend / net / load）——不是每个 tick 三次
    expect(echarts.setOption).toHaveBeenCalledTimes(3)
  })

  it('窗口结束后的新 tick 可再次触发刷新', async () => {
    mountView()
    await settle()
    push({ type: 'tick', data: snap({ timestamp: 1 }) })
    vi.advanceTimersByTime(1000)
    await nextTick()
    push({ type: 'tick', data: snap({ timestamp: 2 }) })
    echarts.setOption.mockClear()
    vi.advanceTimersByTime(1000)
    await nextTick()
    expect(echarts.setOption).toHaveBeenCalledTimes(3)
  })

  it('只有 1 个数据点时两张折线图不画、仪表盘照常更新', async () => {
    mountView()
    await settle()
    echarts.setOption.mockClear()
    push({ type: 'init', data: [snap({ timestamp: 1 })] })
    vi.advanceTimersByTime(1000)
    await nextTick()
    // 折线图需 >= 2 点 → 只刷仪表盘 1 次
    expect(echarts.setOption).toHaveBeenCalledTimes(1)
  })

  it('主题切换分发到三个面板的 applyTheme（补丁不重建）', async () => {
    mountView()
    await settle()
    echarts.applyOption.mockClear()
    document.documentElement.classList.add('dark')
    await settle()
    await nextTick()
    const calls = echarts.applyOption.mock.calls.length
    // 三个面板各补一次；MutationObserver 可能把一次变更拆成多批投递
    expect(calls).toBeGreaterThanOrEqual(3)
    expect(calls % 3).toBe(0)
    document.documentElement.classList.remove('dark')
    await settle()
  })

  it('组件卸载：取消待刷新任务并停止轮询', async () => {
    const w = mountView()
    await settle()
    push({ type: 'tick', data: snap() })
    w.unmount()
    echarts.setOption.mockClear()
    vi.advanceTimersByTime(2000)
    expect(echarts.setOption).not.toHaveBeenCalled()
    expect(polling.stop).toHaveBeenCalled()
  })
})

describe('DashboardView · 时间范围（S-16）', () => {
  const switchRange = async (w: ReturnType<typeof mountView>, range: TrendRange) => {
    w.findComponent(TrendPanel).vm.$emit('update:range', range)
    await nextTick()
  }

  it('默认实时：范围控件可见、历史查询拿到 live（composable 内部据此禁用请求）', async () => {
    const w = mountView()
    await settle()
    expect(w.find('[data-tour="dashboard-range"]').exists()).toBe(true)
    expect(metricsHistory.range?.value).toBe('live')
    expect(w.find('.panel-trend .ws-badge').exists()).toBe(true)
  })

  it('切到 24h：范围下发到历史查询并持久化；两张折线图各打一次时间轴补丁', async () => {
    const w = mountView()
    await settle()
    echarts.applyOption.mockClear()
    await switchRange(w, '24h')
    expect(metricsHistory.range?.value).toBe('24h')
    expect(localStorage.getItem(RANGE_KEY)).toBe('24h')
    // 趋势图 1 h / 网络图（窄图）2 h：两张图各一次，且补丁只含 xAxis
    const intervals = echarts.applyOption.mock.calls
      .map(([opt]) => opt as { xAxis?: { minInterval?: number } })
      .filter((opt) => Object.keys(opt).length === 1 && opt.xAxis?.minInterval !== undefined)
      .map((opt) => opt.xAxis!.minInterval)
      .sort((a, b) => a! - b!)
    expect(intervals).toEqual([60 * 60_000, 2 * 60 * 60_000])
  })

  it('历史样本到达 → 走共享节流器重绘（不是每张图各自 watch 数据）', async () => {
    const w = mountView()
    await settle()
    await switchRange(w, '24h')
    vi.advanceTimersByTime(1000)
    await nextTick()
    echarts.setOption.mockClear()
    metricsHistory.samples!.value = [
      historyPoint(1_700_000_000_000),
      historyPoint(1_700_000_060_000),
    ]
    await nextTick()
    // 窗口未到期：一次都没刷
    expect(echarts.setOption).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1000)
    await nextTick()
    // 到点后三张图各刷一次；折线图吃的是历史样本（[ms, 值] 对）
    expect(echarts.setOption).toHaveBeenCalledTimes(3)
    const trendCall = echarts.setOption.mock.calls.find(
      ([opt]) => ((opt as { series?: unknown[] }).series?.length ?? 0) === 3,
    )
    expect(trendCall).toBeDefined()
    const [[firstPoint]] = (trendCall![0] as { series: { data: [number, number][] }[] }).series.map(
      (s) => s.data,
    )
    expect(firstPoint).toEqual([1_700_000_000_000, 1])
  })

  it('历史模式下 WS tick 只更新指标卡 / 仪表盘，折线图仍画历史样本', async () => {
    const w = mountView()
    await settle()
    await switchRange(w, '1h')
    metricsHistory.samples!.value = [
      historyPoint(1_700_000_000_000),
      historyPoint(1_700_000_060_000),
    ]
    vi.advanceTimersByTime(1000)
    await nextTick()
    echarts.setOption.mockClear()
    push({
      type: 'init',
      data: [snap({ timestamp: 1, cpu_usage: 90 }), snap({ timestamp: 2, cpu_usage: 91 })],
    })
    vi.advanceTimersByTime(1000)
    await nextTick()
    expect(w.find('.stat-value').text()).toBe('91.0%')
    const trendCall = echarts.setOption.mock.calls.find(
      ([opt]) => ((opt as { series?: unknown[] }).series?.length ?? 0) === 3,
    )
    const cpuSeries = (trendCall![0] as { series: { data: [number, number][] }[] }).series[0].data
    expect(cpuSeries.map(([, v]) => v)).toEqual([1, 1])
  })

  it('切回实时：折线图回到 WS 历史，徽标恢复 WS 连接状态', async () => {
    const w = mountView()
    await settle()
    push({
      type: 'init',
      data: [snap({ timestamp: 1_700_000_000_000 }), snap({ timestamp: 1_700_000_001_000 })],
    })
    await switchRange(w, '7d')
    await switchRange(w, 'live')
    expect(metricsHistory.range?.value).toBe('live')
    expect(localStorage.getItem(RANGE_KEY)).toBe('live')
    vi.advanceTimersByTime(1000)
    await nextTick()
    const trendCall = echarts.setOption.mock.calls.find(
      ([opt]) => ((opt as { series?: unknown[] }).series?.length ?? 0) === 3,
    )
    const cpuSeries = (trendCall![0] as { series: { data: [number, number][] }[] }).series[0].data
    expect(cpuSeries.map(([ms]) => ms)).toEqual([1_700_000_000_000, 1_700_000_001_000])
  })

  it('重进页面沿用上次选择的范围；非法存储值回落实时', async () => {
    localStorage.setItem(RANGE_KEY, '7d')
    let w = mountView()
    await settle()
    expect(metricsHistory.range?.value).toBe('7d')
    w.unmount()

    localStorage.setItem(RANGE_KEY, '30d')
    w = mountView()
    await settle()
    expect(metricsHistory.range?.value).toBe('live')
    w.unmount()
  })
})

describe('DashboardView · 节点切换（Doc/30 M-P）', () => {
  function mountAs(role: string) {
    const pinia = createPinia()
    setActivePinia(pinia)
    useAuthStore().role = role
    return mount(DashboardView, {
      global: {
        plugins: [pinia, queryPlugin(), createTestI18n(messages), openVuePlugin, ...services],
      },
    })
  }
  afterEach(() => {
    useAuthStore().role = ''
  })

  it('无节点时不显示切换器；本机订阅 /ws/metrics 且进程面板可见', async () => {
    const w = mountAs('admin')
    await settle()
    expect(api.listNodes).toHaveBeenCalledWith(1, 100)
    expect(w.find('.node-switcher').exists()).toBe(false)
    expect(ws.opened.map((o) => o.url)).toEqual(['/ws/metrics'])
    expect(w.text()).toContain('进程 TOP')
  })

  it('无 node:read 的角色不请求节点列表', async () => {
    mountAs('guest')
    await settle()
    expect(api.listNodes).not.toHaveBeenCalled()
  })

  it('?node= 直达远程节点：订阅节点端点、隐藏进程面板与范围切换、固定实时', async () => {
    localStorage.setItem(RANGE_KEY, '7d')
    api.listNodes.mockResolvedValue({
      data: { data: [{ id: 7, name: 'edge-1', ip_address: '10.0.0.7' }], total: 1 },
    })
    nav.route!.query = { node: '7' }
    const w = mountAs('admin')
    await settle()
    expect(ws.opened.map((o) => o.url)).toEqual(['/ws/metrics?node_id=7'])
    expect(w.find('.node-switcher').exists()).toBe(true)
    expect(w.text()).not.toContain('进程 TOP')
    expect(w.find('.range-switch').exists()).toBe(false)
    expect(metricsHistory.range?.value).toBe('live')
  })

  it('切回本机：关闭节点订阅、清空曲线，并移除 URL 中的 node 参数', async () => {
    nav.route!.query = { node: '7' }
    const w = mountAs('admin')
    await settle()
    push({ type: 'tick', data: snap({ cpu_usage: 77 }) })
    await nextTick()
    expect(w.find('.stat-value').text()).toBe('77.0%')

    w.findComponent({ name: 'NodeSwitcher' }).vm.$emit('update:modelValue', null)
    await settle()
    expect(nav.replace).toHaveBeenCalledWith({ query: {} })
    expect(ws.opened[0].close).toHaveBeenCalled()
    expect(ws.opened.map((o) => o.url)).toEqual(['/ws/metrics?node_id=7', '/ws/metrics'])
    // 两台机器的数据不拼接：切换后回到首帧前的骨架
    expect(w.find('.stat-skeleton').exists()).toBe(true)
  })

  it('非法 node 参数按本机处理', async () => {
    nav.route!.query = { node: 'abc' }
    mountAs('admin')
    await settle()
    expect(ws.opened.map((o) => o.url)).toEqual(['/ws/metrics'])
  })
})
