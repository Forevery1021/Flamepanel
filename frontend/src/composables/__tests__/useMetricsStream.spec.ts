import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent, nextTick, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { HISTORY_LIMIT, metricsEndpoint, useMetricsStream } from '@/composables/useMetricsStream'
import type { MetricsSnapshot } from '@/types'

/**
 * T2-4 第五站的数据层单测（`DashboardView` → `useMetricsStream`）。
 *
 * 这条链路是**纯 WS 推送**（REST 只提供进程表），三条语义在拆分前埋在视图
 * 130 行的 onMessage 回调里：
 * ① `init` 覆盖（重连不拼接历史）/ `tick` 追加；
 * ② 历史封顶 120 点（长连接下内存与曲线密度都不能无限涨）；
 * ③ 快照**原地** Object.assign（整体替换会让四张卡与网络面板的引用指向旧值）。
 */

const { wsOptions, opened } = vi.hoisted(() => ({
  wsOptions: { current: null as null | { onMessage: (d: unknown) => void; onStatus?: (c: boolean) => void } },
  /** 每次 useWebSocket 调用：端点 + 生命周期桩（节点切换断言用） */
  opened: [] as { url: string; manageLifecycle?: boolean; connect: ReturnType<typeof vi.fn>; close: ReturnType<typeof vi.fn>; pause: ReturnType<typeof vi.fn>; resume: ReturnType<typeof vi.fn> }[],
}))

vi.mock('@/composables/useWebSocket', () => ({
  useWebSocket: (
    url: string,
    options: { onMessage: (d: unknown) => void; onStatus?: (c: boolean) => void; manageLifecycle?: boolean },
  ) => {
    wsOptions.current = options
    const inst = { url, manageLifecycle: options.manageLifecycle, connect: vi.fn(), close: vi.fn(), pause: vi.fn(), resume: vi.fn() }
    opened.push(inst)
    return {
      connect: inst.connect,
      close: inst.close,
      pause: inst.pause,
      resume: inst.resume,
      send: vi.fn(),
      reconnectNow: vi.fn(),
      isActive: () => true,
    }
  },
}))

const snap = (over: Partial<MetricsSnapshot> = {}): MetricsSnapshot => ({
  timestamp: 1700000000,
  cpu_usage: 12.3,
  cpu_cores: 8,
  memory_usage_percent: 63.8,
  memory_total_mb: 16384,
  memory_used_mb: 8192,
  disk_usage_percent: 44.2,
  disk_total_gb: 100,
  disk_used_gb: 44.2,
  load_one: 3.456,
  load_five: 2.5,
  load_fifteen: 1.25,
  network_rx_mbps: 1.234,
  network_tx_mbps: 0.5,
  ...over,
})

function mountCtl(options: { onTick?: () => void } = {}) {
  const Host = defineComponent({
    setup() {
      return { ctl: useMetricsStream(options) }
    },
    template: '<div/>',
  })
  const w = mount(Host)
  return (w.vm as unknown as { ctl: ReturnType<typeof useMetricsStream> }).ctl
}

/** 取最近一次挂载传给 useWebSocket 的回调 */
function push(data: unknown) {
  wsOptions.current!.onMessage(data)
}

beforeEach(() => {
  wsOptions.current = null
  opened.length = 0
})

describe('useMetricsStream · init / tick 语义', () => {
  it('init 覆盖历史（重连不把两段历史拼起来）', () => {
    const ctl = mountCtl()
    push({ type: 'init', data: [snap({ cpu_usage: 1 }), snap({ cpu_usage: 2 })] })
    expect(ctl.history.value).toHaveLength(2)
    push({ type: 'init', data: [snap({ cpu_usage: 9 })] })
    expect(ctl.history.value).toHaveLength(1)
    expect(ctl.history.value[0].cpu_usage).toBe(9)
  })

  it('tick 追加历史并原地更新快照', () => {
    const ctl = mountCtl()
    push({ type: 'init', data: [snap({ cpu_usage: 1 })] })
    push({ type: 'tick', data: snap({ cpu_usage: 42.5, memory_usage_percent: 50 }) })
    expect(ctl.history.value).toHaveLength(2)
    expect(ctl.snap.cpu_usage).toBe(42.5)
    expect(ctl.snap.memory_usage_percent).toBe(50)
  })

  it('快照是原地更新（引用不变——整体替换会让指标卡与网络面板读到旧值）', () => {
    const ctl = mountCtl()
    const ref = ctl.snap
    push({ type: 'tick', data: snap({ cpu_usage: 77 }) })
    expect(ctl.snap).toBe(ref)
    expect(ref.cpu_usage).toBe(77)
  })

  it('未知消息类型被忽略（不抛错、不改状态）', () => {
    const ctl = mountCtl()
    push({ type: 'tick', data: snap({ cpu_usage: 12.3 }) })
    push({ type: 'unknown-type', data: snap({ cpu_usage: 99 }) })
    expect(ctl.history.value).toHaveLength(1)
    // 未知类型不改快照：仍是上一次 tick 的值（99 未被采纳）
    expect(ctl.snap.cpu_usage).toBe(12.3)
  })

  it('tick 会更新「最后更新」与触发 onTick 回调（节流入口）', () => {
    const onTick = vi.fn()
    const ctl = mountCtl({ onTick })
    expect(ctl.lastUpdate.value).toBe('')
    push({ type: 'tick', data: snap() })
    expect(ctl.lastUpdate.value).not.toBe('')
    expect(onTick).toHaveBeenCalled()
  })

  it('init 也触发 onTick（首屏就要画曲线）', () => {
    const onTick = vi.fn()
    const ctl = mountCtl({ onTick })
    push({ type: 'init', data: [snap(), snap()] })
    expect(onTick).toHaveBeenCalled()
    expect(ctl.history.value).toHaveLength(2)
  })
})

describe('useMetricsStream · 开屏优化', () => {
  it('init 同时回填快照（指标卡不必等下一个 tick 才离开 0%）', () => {
    const ctl = mountCtl()
    expect(ctl.hasData.value).toBe(false)
    push({ type: 'init', data: [snap({ cpu_usage: 1 }), snap({ cpu_usage: 33, load_one: 2.5 })] })
    expect(ctl.snap.cpu_usage).toBe(33)
    expect(ctl.snap.load_one).toBe(2.5)
    expect(ctl.hasData.value).toBe(true)
    expect(ctl.lastUpdate.value).not.toBe('')
  })

  it('空 init 不改快照、不标记有数据', () => {
    const ctl = mountCtl()
    push({ type: 'init', data: [] })
    expect(ctl.hasData.value).toBe(false)
    expect(ctl.snap.cpu_usage).toBe(0)
  })

  it('history 是浅响应 + 整体替换：tick 后引用变化、元素保持原生对象', () => {
    const ctl = mountCtl()
    push({ type: 'init', data: [snap({ cpu_usage: 1 })] })
    const before = ctl.history.value
    push({ type: 'tick', data: snap({ cpu_usage: 2 }) })
    expect(ctl.history.value).not.toBe(before)
    expect(before).toHaveLength(1) // 旧数组未被原地修改
    expect(ctl.history.value).toHaveLength(2)
  })

  it(`超长 init 只保留最近 ${HISTORY_LIMIT} 点`, () => {
    const ctl = mountCtl()
    const data = Array.from({ length: HISTORY_LIMIT + 5 }, (_, i) => snap({ cpu_usage: i }))
    push({ type: 'init', data })
    expect(ctl.history.value).toHaveLength(HISTORY_LIMIT)
    expect(ctl.history.value[0].cpu_usage).toBe(5)
  })
})

describe('useMetricsStream · 历史封顶', () => {
  it(`超过 ${HISTORY_LIMIT} 点时丢最旧（内存与曲线密度都不无限涨）`, () => {
    const ctl = mountCtl()
    for (let i = 0; i < HISTORY_LIMIT + 10; i++) {
      push({ type: 'tick', data: snap({ cpu_usage: i }) })
    }
    expect(ctl.history.value).toHaveLength(HISTORY_LIMIT)
    // 保留的是最近一段：首元素是第 10 个 tick
    expect(ctl.history.value[0].cpu_usage).toBe(10)
    expect(ctl.history.value[HISTORY_LIMIT - 1].cpu_usage).toBe(HISTORY_LIMIT + 9)
  })
})

describe('useMetricsStream · 连接状态', () => {
  it('onStatus 同步到 wsConnected（徽标绿点）', () => {
    const ctl = mountCtl()
    expect(ctl.wsConnected.value).toBe(false)
    wsOptions.current!.onStatus!(true)
    expect(ctl.wsConnected.value).toBe(true)
    wsOptions.current!.onStatus!(false)
    expect(ctl.wsConnected.value).toBe(false)
  })

  it('connect 转发到底层 WS 客户端', () => {
    const ctl = mountCtl()
    expect(() => ctl.connect()).not.toThrow()
    expect(ctl.wsConnected.value).toBe(false)
  })
})

describe('useMetricsStream · 节点切换（Doc/30 M-P）', () => {
  function mountWithNode(initial: number | null) {
    const nodeId = ref<number | null>(initial)
    const Host = defineComponent({
      setup() {
        return { ctl: useMetricsStream({ nodeId }) }
      },
      template: '<div/>',
    })
    const w = mount(Host)
    const ctl = (w.vm as unknown as { ctl: ReturnType<typeof useMetricsStream> }).ctl
    return { w, ctl, nodeId }
  }

  it('端点：本机 / 远程节点', () => {
    expect(metricsEndpoint()).toBe('/ws/metrics')
    expect(metricsEndpoint(null)).toBe('/ws/metrics')
    expect(metricsEndpoint(3)).toBe('/ws/metrics?node_id=3')
  })

  it('生命周期由本层统一挂接（useWebSocket 不自挂钩子），卸载关闭当前端点', () => {
    const { w, ctl } = mountWithNode(5)
    expect(opened).toHaveLength(1)
    expect(opened[0].url).toBe('/ws/metrics?node_id=5')
    expect(opened[0].manageLifecycle).toBe(false)
    ctl.connect()
    expect(opened[0].connect).toHaveBeenCalledTimes(1)
    w.unmount()
    expect(opened[0].close).toHaveBeenCalled()
  })

  it('切换节点：关闭旧订阅、清空历史与快照，已启动则立即连新端点', async () => {
    const { ctl, nodeId } = mountWithNode(null)
    ctl.connect()
    push({ type: 'init', data: [snap({ cpu_usage: 40 })] })
    expect(ctl.hasData.value).toBe(true)

    nodeId.value = 9
    await nextTick()
    expect(opened[0].close).toHaveBeenCalled()
    expect(opened[1].url).toBe('/ws/metrics?node_id=9')
    expect(opened[1].connect).toHaveBeenCalledTimes(1)
    expect(ctl.history.value).toEqual([])
    expect(ctl.snap.cpu_usage).toBe(0)
    expect(ctl.hasData.value).toBe(false)
    expect(ctl.wsConnected.value).toBe(false)
    expect(ctl.lastUpdate.value).toBe('')
  })

  it('尚未 connect 时切换节点只重建、不提前连接', async () => {
    const { nodeId } = mountWithNode(null)
    nodeId.value = 2
    await nextTick()
    expect(opened[1].url).toBe('/ws/metrics?node_id=2')
    expect(opened[1].connect).not.toHaveBeenCalled()
  })
})
