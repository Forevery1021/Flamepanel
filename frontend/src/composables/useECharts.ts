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
    // S-16 修正：主题补丁必须**合并**进现有 option。此前 `notMerge: true` 会把整份 option
    // 替换成只含轴配色的补丁——series 定义随之丢失，主题切换后三张图永久空白并持续报
    // “Unknown series”。补丁只含轴 / 图例 / 数值配色（见 dashboardCharts 的 *ThemePatch），
    // 合并模式下不会残留旧配置。
    if (chart) chart.setOption(option, { notMerge: false, lazyUpdate: true })
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
