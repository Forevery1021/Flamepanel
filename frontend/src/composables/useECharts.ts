import { onBeforeUnmount, onDeactivated, onActivated, type Ref } from 'vue'
import { init, type ECharts, type EChartsCoreOption } from 'echarts/core'

export interface UseEChartsInstance {
  /** 初始化并设置首屏 option（惰性，DOM 就绪后调用） */
  mount: (option?: EChartsCoreOption) => ECharts | null
  /** 更新图表（keep-alive 失活时跳过，避免后台无谓渲染） */
  setOption: (option: EChartsCoreOption, lazyUpdate?: boolean) => void
  /** 触发 resize */
  resize: () => void
  /** 应用主题/颜色变化（强制刷新，即使后台也立即生效） */
  applyOption: (option: EChartsCoreOption) => void
  /** 暂停后台更新（onDeactivated 自动调用，也可手动） */
  pause: () => void
  /** 恢复更新（onActivated 自动调用，含 resize） */
  resume: () => void
  /** 手动销毁实例 */
  dispose: () => void
  /** 底层实例（调试/高级用） */
  instance: () => ECharts | null
}

/**
 * useECharts — 统一 ECharts 实例生命周期管理。
 * - 组件卸载自动 dispose
 * - keep-alive 失活（onDeactivated）自动暂停 setOption，恢复（onActivated）自动 resume + resize
 * - 主题切换等场景用 applyOption 强制刷新
 */
export function useECharts(el: Ref<HTMLElement | undefined>): UseEChartsInstance {
  let chart: ECharts | null = null
  let active = true

  function mount(option?: EChartsCoreOption): ECharts | null {
    if (chart || !el.value) return chart
    chart = init(el.value)
    if (option) chart.setOption(option)
    return chart
  }

  function setOption(option: EChartsCoreOption, lazyUpdate = false) {
    // F1.3：高频更新使用 lazyUpdate（把渲染合并进下一帧），避免长任务尖刺
    if (active && chart) chart.setOption(option, { lazyUpdate })
  }

  function applyOption(option: EChartsCoreOption) {
    // F1.3：主题切换等一次性刷新用 notMerge，避免残留旧配置
    if (chart) chart.setOption(option, { notMerge: true, lazyUpdate: true })
  }

  function resize() {
    chart?.resize()
  }

  function pause() {
    active = false
  }

  function resume() {
    active = true
    chart?.resize()
  }

  function dispose() {
    chart?.dispose()
    chart = null
  }

  onActivated(() => resume())
  onDeactivated(() => pause())
  onBeforeUnmount(() => dispose())

  return { mount, setOption, resize, applyOption, pause, resume, dispose, instance: () => chart }
}
