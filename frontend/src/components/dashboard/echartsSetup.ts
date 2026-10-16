/**
 * ECharts 按需注册（开屏优化）。
 *
 * 旧实现把 `use([...])` 放在 DashboardView 顶层，等于让**仪表盘路由块静态依赖
 * echarts 分包**——浏览器必须先下载并解析 ~110KB(gzip) 的 echarts，才能挂载整个
 * 仪表盘，四张指标卡也被一起拖到 echarts 之后才出现。
 *
 * 现在注册跟着三块图表面板走：面板经 `defineAsyncComponent` 懒加载，本模块与
 * echarts 一起落在异步块里；指标卡 / 进程 / 待办 / 常用应用在首帧即可渲染，
 * 图表就绪后原位替换骨架。`use()` 幂等，三个面板各自 import 也只注册一次。
 */
import { use } from 'echarts/core'
import { LineChart, GaugeChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'

let registered = false

export function registerDashboardCharts(): void {
  if (registered) return
  registered = true
  use([LineChart, GaugeChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])
}
