import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import TrendPanel from '@/components/dashboard/TrendPanel.vue'
import NetworkPanel from '@/components/dashboard/NetworkPanel.vue'
import LoadPanel from '@/components/dashboard/LoadPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import type { MetricsSnapshot } from '@/types'

/**
 * T2-4 第五站三张图表面板的接线契约。
 *
 * jsdom 没有画布，ECharts 实例不会真的绘图——故此处**不测绘图结果**（那是
 * `dashboardCharts.spec` 的纯函数职责），只测三件接线事实：
 * ① 面板挂载即装配一次首屏 option（空数据 + 当前配色）；
 * ② 重绘由 `refreshKey` 驱动而非数据本身（节流器在视图层，面板不该自行响应 tick）；
 * ③ 折线图「不足两点不画」的判据就位。
 */

const { echarts } = vi.hoisted(() => ({
  echarts: {
    mount: vi.fn(),
    setOption: vi.fn(),
    applyOption: vi.fn(),
    instance: vi.fn(() => ({ __fake: true })),
  },
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
    trend: '性能趋势',
    wsConnected: '已连接',
    wsDisconnected: '未连接',
    network: '网络 IO',
    rx: '下行',
    tx: '上行',
    load: '系统负载',
    dataPoints: '数据点',
    lastUpdate: '最后更新',
  },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(component: unknown, props: Record<string, unknown> = {}) {
  return mountLoose(component, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
    },
  })
}

const snap = (over: Partial<MetricsSnapshot> = {}): MetricsSnapshot => ({
  timestamp: 1,
  cpu_usage: 10,
  cpu_cores: 4,
  memory_usage_percent: 20,
  memory_total_mb: 1024,
  memory_used_mb: 512,
  disk_usage_percent: 30,
  disk_total_gb: 10,
  disk_used_gb: 3,
  load_one: 1.234,
  load_five: 1,
  load_fifteen: 0.5,
  network_rx_mbps: 0.5,
  network_tx_mbps: 0.25,
  ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  echarts.instance.mockReturnValue({ __fake: true })
})

describe('TrendPanel · 趋势面板', () => {
  it('挂载即装配首屏 option（空数据、当前配色）', async () => {
    mountPanel(TrendPanel, { history: [], wsConnected: true, refreshKey: 0 })
    await nextTick()
    expect(echarts.mount).toHaveBeenCalledTimes(1)
  })

  it('WS 徽标按连接状态切换文案与点位', () => {
    const w = mountPanel(TrendPanel, { history: [], wsConnected: true, refreshKey: 0 })
    expect(w.find('.ws-badge').text()).toContain('已连接')
    expect(w.find('.dot').classes()).toContain('green')
  })

  it('未连接：红点 + 未连接文案', () => {
    const w = mountPanel(TrendPanel, { history: [], wsConnected: false, refreshKey: 0 })
    expect(w.find('.ws-badge').text()).toContain('未连接')
    expect(w.find('.dot').classes()).toContain('red')
  })

  it('refreshKey 递增触发重绘（数据够两个点）', async () => {
    const w = mountPanel(TrendPanel, {
      history: [snap({ timestamp: 1 }), snap({ timestamp: 2 })],
      wsConnected: true,
      refreshKey: 0,
    })
    echarts.setOption.mockClear()
    await w.setProps({ refreshKey: 1 })
    expect(echarts.setOption).toHaveBeenCalledTimes(1)
  })

  it('只有 1 个数据点时**不**重绘（折线图会退化成孤点）', async () => {
    const w = mountPanel(TrendPanel, { history: [snap()], wsConnected: true, refreshKey: 0 })
    echarts.setOption.mockClear()
    await w.setProps({ refreshKey: 1 })
    expect(echarts.setOption).not.toHaveBeenCalled()
  })

  it('实例为 null（DOM 未就绪）时不重绘、不抛错', async () => {
    echarts.instance.mockReturnValue(null as unknown as { __fake: boolean })
    const w = mountPanel(TrendPanel, {
      history: [snap(), snap()],
      wsConnected: true,
      refreshKey: 0,
    })
    echarts.setOption.mockClear()
    await w.setProps({ refreshKey: 1 })
    expect(echarts.setOption).not.toHaveBeenCalled()
  })

  it('applyTheme 走 applyOption 补丁（不整份重建，历史曲线不丢）', async () => {
    const w = mountPanel(TrendPanel, { history: [snap()], wsConnected: true, refreshKey: 0 })
    ;(w.vm as unknown as { applyTheme: () => void }).applyTheme()
    expect(echarts.applyOption).toHaveBeenCalledTimes(1)
    expect(echarts.setOption).not.toHaveBeenCalled()
  })
})

describe('NetworkPanel · 网络面板', () => {
  it('两个实时读数按两位小数渲染（MB/s）', () => {
    const w = mountPanel(NetworkPanel, {
      history: [],
      rxMbps: 1.239,
      txMbps: 0.5,
      refreshKey: 0,
    })
    const vals = w.findAll('.net-val')
    expect(vals[0].text()).toBe('下行 1.24 MB/s')
    expect(vals[1].text()).toBe('上行 0.50 MB/s')
  })

  it('读数为 0 时正常渲染（不是空串或 NaN）', () => {
    const w = mountPanel(NetworkPanel, { history: [], rxMbps: 0, txMbps: 0, refreshKey: 0 })
    expect(w.find('.net-val').text()).toBe('下行 0.00 MB/s')
  })

  it('refreshKey 递增触发重绘（两个点）', async () => {
    const w = mountPanel(NetworkPanel, {
      history: [snap({ timestamp: 1 }), snap({ timestamp: 2 })],
      rxMbps: 0,
      txMbps: 0,
      refreshKey: 0,
    })
    echarts.setOption.mockClear()
    await w.setProps({ refreshKey: 1 })
    expect(echarts.setOption).toHaveBeenCalledTimes(1)
  })
})

describe('LoadPanel · 负载面板', () => {
  it('数据点数与最后更新如实渲染', () => {
    const w = mountPanel(LoadPanel, {
      dataPoints: 42,
      lastUpdate: '2026-09-15 12:00:00',
      snapshot: snap(),
      refreshKey: 0,
    })
    expect(w.findAll('.info-row').map((r) => r.text())).toEqual([
      '数据点42',
      '最后更新2026-09-15 12:00:00',
    ])
  })

  it('最后更新为空回落 `—`（空单元格会被读成加载失败）', () => {
    const w = mountPanel(LoadPanel, {
      dataPoints: 0,
      lastUpdate: '',
      snapshot: snap(),
      refreshKey: 0,
    })
    expect(w.findAll('.info-row')[1].text()).toBe('最后更新—')
  })

  it('单值图不受「两个点」限制：refreshKey 一到即更新', async () => {
    const w = mountPanel(LoadPanel, {
      dataPoints: 1,
      lastUpdate: '',
      snapshot: snap(),
      refreshKey: 0,
    })
    echarts.setOption.mockClear()
    await w.setProps({ refreshKey: 1 })
    expect(echarts.setOption).toHaveBeenCalledTimes(1)
  })
})
