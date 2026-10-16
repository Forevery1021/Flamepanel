/**
 * 图表刷新节流（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 仪表盘三张图原本各自 setOption，频率由 WS 推送决定——后端推送间隔可能 < 1s，
 * 每个 tick 都重绘会在低端机产生长任务尖刺（F1.3 已记录）。这三张图共享同一份
 * 快照与同一个可见性状态，故节流窗口也**只有一个**：
 * 1s 内最多合并刷新一次三图，窗口内到达的后续 tick 直接丢弃（不是排队补发，
 * 曲线取的是最新一轮快照，补发旧帧反而让曲线回退）。
 *
 * 后台标签页（`document.visibilityState !== 'visible'`）：
 * - 标记 `pending` 并跳过刷新（后台算图表毫无意义）；
 * - 回到前台若积压了更新，立即补一次，避免用户切回来看到旧曲线。
 *
 * 边界：本 composable 只依赖一个回调与 `document.visibilityState`，
 * 不 import echarts / 不接触具体图表——便于单测锁死节流与补刷时机。
 */

/** 默认节流窗口（ms） */
export const CHART_THROTTLE_MS = 1000

export function useChartThrottle(refresh: () => void, delayMs: number = CHART_THROTTLE_MS) {
  let timer: number | null = null
  let pending = false

  function isVisible(): boolean {
    return document.visibilityState === 'visible'
  }

  /**
   * 节流入口：窗口内只排一次刷新。
   *
   * 计时结束时**重新判可见性**：窗口期里切到后台则本次丢弃并清空 pending
   * （原实现如此）——后续 tick 会重新打上 pending，回前台时自然补刷。
   */
  function schedule() {
    if (!isVisible()) {
      pending = true
      return
    }
    if (timer === null) {
      timer = window.setTimeout(() => {
        timer = null
        pending = false
        if (!isVisible()) return
        refresh()
      }, delayMs)
    }
  }

  /** 页面恢复可见时若积压了更新，立即补一次（并取消已排队的定时器） */
  function onVisibilityChange() {
    if (isVisible() && pending) {
      cancel()
      pending = false
      refresh()
    }
  }

  function cancel() {
    if (timer !== null) {
      window.clearTimeout(timer)
      timer = null
    }
  }

  return { schedule, onVisibilityChange, cancel }
}
