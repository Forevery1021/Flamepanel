import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { useChartThrottle, CHART_THROTTLE_MS } from '@/composables/useChartThrottle'

/**
 * T2-4 第五站的节流器单测（`DashboardView` → `useChartThrottle`）。
 *
 * 仪表盘三张图共享这一份节流：后端 WS 推送间隔可能 < 1s，若每个 tick 都重绘，
 * 低端机会出现长任务尖刺。三条行为必须锁死：
 * ① 窗口内多次 schedule 只触发一次 refresh（合并，不是排队补发）；
 * ② 后台标签页不刷新，但标记 pending，回前台立即补一次；
 * ③ cancel 之后不再触发（组件卸载路径）。
 */

/** 可控的可见性开关：jsdom 的 visibilityState 是只读的，用 defineProperty 覆盖 */
function setVisibility(state: 'visible' | 'hidden') {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    get: () => state,
  })
}

function mountCtl(refresh = vi.fn()) {
  const Host = defineComponent({
    setup() {
      return { ctl: useChartThrottle(refresh) }
    },
    template: '<div/>',
  })
  const w = mount(Host)
  return { ctl: (w.vm as unknown as { ctl: ReturnType<typeof useChartThrottle> }).ctl, refresh }
}

beforeEach(() => {
  vi.useFakeTimers()
  setVisibility('visible')
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useChartThrottle · 节流合并', () => {
  it('窗口内多次 schedule 只刷新一次（合并而非排队）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    ctl.schedule()
    ctl.schedule()
    expect(refresh).not.toHaveBeenCalled()
    vi.advanceTimersByTime(CHART_THROTTLE_MS)
    expect(refresh).toHaveBeenCalledTimes(1)
  })

  it('窗口结束后可再次排入（下一个窗口照常刷新）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    vi.advanceTimersByTime(CHART_THROTTLE_MS)
    ctl.schedule()
    vi.advanceTimersByTime(CHART_THROTTLE_MS)
    expect(refresh).toHaveBeenCalledTimes(2)
  })

  it('恰好 999ms 不触发（窗口是 1000ms，不是「差不多」）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    vi.advanceTimersByTime(CHART_THROTTLE_MS - 1)
    expect(refresh).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(refresh).toHaveBeenCalledTimes(1)
  })
})

describe('useChartThrottle · 可见性', () => {
  it('后台标签页 schedule 不刷新（不算图表）', () => {
    const { ctl, refresh } = mountCtl()
    setVisibility('hidden')
    ctl.schedule()
    vi.advanceTimersByTime(CHART_THROTTLE_MS * 3)
    expect(refresh).not.toHaveBeenCalled()
  })

  it('回到前台立即补刷一次（用户切回来不该看到旧曲线）', () => {
    const { ctl, refresh } = mountCtl()
    setVisibility('hidden')
    ctl.schedule()
    setVisibility('visible')
    ctl.onVisibilityChange()
    expect(refresh).toHaveBeenCalledTimes(1)
  })

  it('前台状态下的 onVisibilityChange 不额外刷新（无积压则不动）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.onVisibilityChange()
    expect(refresh).not.toHaveBeenCalled()
  })

  it('补刷会取消已排队的定时器（不重复刷两次）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule() // 前台排入一次
    setVisibility('hidden')
    ctl.schedule() // 后台 → pending
    setVisibility('visible')
    ctl.onVisibilityChange() // 立即补刷
    vi.advanceTimersByTime(CHART_THROTTLE_MS * 2)
    // 补刷 1 次；被取消的定时器不应再触发
    expect(refresh).toHaveBeenCalledTimes(1)
  })

  it('定时器到期时已切到后台：本次丢弃，不刷新', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    setVisibility('hidden')
    vi.advanceTimersByTime(CHART_THROTTLE_MS)
    expect(refresh).not.toHaveBeenCalled()
  })
})

describe('useChartThrottle · cancel', () => {
  it('cancel 后已排队的刷新不再触发（卸载路径）', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    ctl.cancel()
    vi.advanceTimersByTime(CHART_THROTTLE_MS * 2)
    expect(refresh).not.toHaveBeenCalled()
  })

  it('cancel 不影响后续重新 schedule', () => {
    const { ctl, refresh } = mountCtl()
    ctl.schedule()
    ctl.cancel()
    ctl.schedule()
    vi.advanceTimersByTime(CHART_THROTTLE_MS)
    expect(refresh).toHaveBeenCalledTimes(1)
  })
})
