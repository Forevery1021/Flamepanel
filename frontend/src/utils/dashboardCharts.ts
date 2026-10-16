/**
 * 仪表盘图表 option 纯函数（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 拆分前 851 行的 DashboardView 里，三张图（性能趋势 / 网络 IO / 负载仪表盘）
 * 的 option 组装与「主题切换补丁」全部内联在视图的 init/update 函数中——这些
 * 结构是**纯数据**（只在调色板与快照上取值），与 ECharts 实例生命周期无关。
 * 抽成纯函数后既可脱离 DOM 单测（jsdom 没有画布，原本这段逻辑一行都测不到），
 * 也避免「初始化 option」与「增量 option」两处各写一份系列名/配色而慢慢漂移。
 *
 * 边界：本文件不 import vue / 不触碰 DOM；调色板一律由调用方传参
 * （`ChartPalette` 的取值见 `composables/useMetricsCharts.ts` 的 CSS 变量读取）。
 */

import type { EChartsCoreOption } from 'echarts/core'
import type { MetricsSnapshot } from '@/types'
import type { MetricsHistoryPoint } from '@/api/generated'

/** 曲线图只读取快照的这几个字段；WS 实时快照与 S-07 历史点映射后都满足此形状 */
export type TrendSample = Pick<
  MetricsSnapshot,
  | 'timestamp'
  | 'cpu_usage'
  | 'memory_usage_percent'
  | 'disk_usage_percent'
  | 'network_rx_mbps'
  | 'network_tx_mbps'
>

/** 趋势图时间范围：`live` 为 WS 实时 6 min，其余走 `GET /api/metrics/history?range=` */
export const TREND_RANGES = ['live', '1h', '24h', '7d'] as const
export type TrendRange = (typeof TREND_RANGES)[number]

/**
 * 时间戳归一为毫秒。后端 `MetricsSnapshot.timestamp` 为毫秒（`timestamp_millis()`），
 * 旧实现按秒 ×1000 后落在 Date 可表示范围之外（Invalid Date），加上把 `xAxis.data`
 * 用在 `type: 'time'` 轴上（时间轴会忽略它），效果是曲线按下标排布、时间刻度从不出现、
 * tooltip 显示 1970 年。≤ 1e11 的值视为秒（测试桩 / 历史点的 `bucket_start`）。
 */
export function toMillis(ts: number): number {
  return ts > 1e11 ? ts : ts * 1000
}

/**
 * 时间刻度格式按范围自适应：实时到秒、小时级到分、天级带日期。
 * `compact` 为窄图（网络 IO 面板）：实时也只到分，避免刻度互相遮挡。
 */
export function timeAxisFormatter(range: TrendRange, compact = false): string {
  switch (range) {
    case 'live':
      return compact ? '{HH}:{mm}' : '{HH}:{mm}:{ss}'
    case '7d':
      return compact ? '{MM}-{dd}' : '{MM}-{dd} {HH}:{mm}'
    default:
      return '{HH}:{mm}'
  }
}

/** 时间刻度最小间隔（ms）：避免刻度过密；窄图翻倍 */
export function timeAxisMinInterval(range: TrendRange, compact = false): number {
  const base = (() => {
    switch (range) {
      case 'live':
        return 5_000
      case '1h':
        return 5 * 60_000
      case '24h':
        return 60 * 60_000
      case '7d':
        return 12 * 60 * 60_000
    }
  })()
  return compact ? base * 2 : base
}

/** 切换范围时的轴补丁（合并模式应用，不动 series） */
export function timeAxisPatch(range: TrendRange, compact = false): EChartsCoreOption {
  return {
    xAxis: {
      axisLabel: { formatter: timeAxisFormatter(range, compact) },
      minInterval: timeAxisMinInterval(range, compact),
    },
  }
}

/**
 * S-07 历史降采样点 → 曲线样本（均值口径）。`bucket_start` 为秒，由 `toMillis` 归一。
 */
export function historyToTrendSamples(points: readonly MetricsHistoryPoint[]): TrendSample[] {
  return points.map((p) => ({
    timestamp: toMillis(p.bucket_start),
    cpu_usage: p.cpu_avg ?? 0,
    memory_usage_percent: p.memory_avg ?? 0,
    disk_usage_percent: p.disk_avg ?? 0,
    network_rx_mbps: p.net_rx_avg_mbps ?? 0,
    network_tx_mbps: p.net_tx_avg_mbps ?? 0,
  }))
}

/**
 * 从 CSS 变量读色（OKLCH 设计令牌 → 具体色值，未定义时返回 undefined）。
 *
 * 读取的是 `document.documentElement` 上的令牌，故主题切换后要重新读一次
 * （见 `Composables · useChartThrottle` 与各面板的 `applyTheme`）。
 */
function cssVar(name: string): string | undefined {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || undefined
}

/** 当前主题下的图表调色板（三图共用一份，避免每张图各自读 8 次 CSS 变量） */
export function readChartPalette(): ChartPalette {
  return {
    axisLabel: cssVar('--fp-text-secondary') || cssVar('--fp-text-muted'),
    gridLine: cssVar('--fp-border') || cssVar('--fp-border-strong'),
    text: cssVar('--fp-text-primary') || cssVar('--fp-text-secondary'),
    brand: cssVar('--fp-brand'),
    success: cssVar('--fp-success'),
    info: cssVar('--fp-info'),
    warning: cssVar('--fp-warning'),
    danger: cssVar('--fp-danger'),
    surface: cssVar('--fp-bg-elevated') || cssVar('--fp-bg-surface'),
  }
}

/** 图表调色板（OKLCH 设计令牌 → 具体色值，缺失项回落框架默认） */
export interface ChartPalette {
  axisLabel?: string
  gridLine?: string
  text?: string
  brand?: string
  success?: string
  info?: string
  warning?: string
  danger?: string
  /** 浮层底色（tooltip）；ECharts 默认白底在暗色主题下刺眼且与面板割裂 */
  surface?: string
}

/** tooltip 跟随主题：浮层底色 / 边框 / 文字全部取设计令牌 */
function tooltipStyle(p: ChartPalette) {
  return {
    backgroundColor: p.surface,
    borderColor: p.gridLine,
    textStyle: { color: p.text },
  }
}

/** 各系列的配色（首屏 option 与主题补丁共用，保证两处不漂移） */
function lineColors(width: number, color?: string) {
  return { lineStyle: { width, color }, itemStyle: { color } }
}
const trendColors = (p: ChartPalette) => [p.brand, p.info, p.success]
const netColors = (p: ChartPalette) => [p.info, p.success]
const gaugeBands = (p: ChartPalette) => [
  [0.3, p.success],
  [0.7, p.warning],
  [1, p.danger],
]

/** 图表内部系列名（ECharts 图例用，非 i18n 文案；原实现即硬编码，保持行为一致） */
export const TREND_SERIES = ['CPU %', 'Memory %', 'Disk %'] as const
export const NET_SERIES = ['RX', 'TX'] as const

/** 负载仪表盘量程上限（与 `dashboardFormat.LOAD_GAUGE_MAX` 同源口径：0–10） */
const GAUGE_MAX = 10

/** 趋势/网络图的公共轴样式（两图同构，抽一处避免改一漏一） */
function axisStyle(p: ChartPalette) {
  return {
    xAxis: {
      type: 'time' as const,
      axisLabel: {
        fontSize: 11,
        color: p.axisLabel,
        hideOverlap: true,
        formatter: timeAxisFormatter('live'),
      },
      splitLine: { lineStyle: { color: p.gridLine } },
      minInterval: timeAxisMinInterval('live'),
    },
    yAxis: {
      type: 'value' as const,
      axisLabel: { fontSize: 11, color: p.axisLabel },
      splitLine: { lineStyle: { color: p.gridLine } },
    },
  }
}

/** 趋势图首屏 option（三条线：CPU / 内存 / 磁盘，Y 轴固定 0–100） */
export function trendInitialOption(p: ChartPalette): EChartsCoreOption {
  const { xAxis, yAxis } = axisStyle(p)
  return {
    tooltip: { trigger: 'axis', ...tooltipStyle(p) },
    legend: { data: [...TREND_SERIES], bottom: 0, textStyle: { color: p.axisLabel } },
    grid: { left: 50, right: 20, top: 20, bottom: 40 },
    xAxis,
    yAxis: { ...yAxis, max: 100 },
    series: [
      {
        name: TREND_SERIES[0],
        type: 'line',
        smooth: true,
        showSymbol: false,
        ...lineColors(2, trendColors(p)[0]),
        data: [],
      },
      {
        name: TREND_SERIES[1],
        type: 'line',
        smooth: true,
        showSymbol: false,
        ...lineColors(2, trendColors(p)[1]),
        data: [],
      },
      {
        name: TREND_SERIES[2],
        type: 'line',
        smooth: true,
        showSymbol: false,
        ...lineColors(2, trendColors(p)[2]),
        data: [],
      },
    ],
  }
}

/**
 * 趋势图增量 option：三系列数据点为 `[毫秒时间, 值]` 二元组。
 *
 * 时间轴（`type: 'time'`）的横坐标必须放在 series 数据里，`xAxis.data` 只对类目轴有效；
 * 旧实现把时间塞进 `xAxis.data`，时间刻度从未真正出现过（见 `toMillis` 注释）。
 */
export function trendUpdateOption(history: readonly TrendSample[]): EChartsCoreOption {
  return {
    series: [
      { data: history.map((s) => [toMillis(s.timestamp), s.cpu_usage]) },
      { data: history.map((s) => [toMillis(s.timestamp), s.memory_usage_percent]) },
      { data: history.map((s) => [toMillis(s.timestamp), s.disk_usage_percent]) },
    ],
  }
}

/** 网络图首屏 option（RX / TX 双曲线） */
export function netInitialOption(p: ChartPalette): EChartsCoreOption {
  return {
    tooltip: { trigger: 'axis', ...tooltipStyle(p) },
    legend: { data: [...NET_SERIES], bottom: 0, textStyle: { color: p.axisLabel } },
    grid: { left: 44, right: 12, top: 16, bottom: 36 },
    xAxis: {
      type: 'time',
      axisLabel: {
        fontSize: 10,
        color: p.axisLabel,
        hideOverlap: true,
        formatter: timeAxisFormatter('live', true),
      },
      splitLine: { lineStyle: { color: p.gridLine } },
      minInterval: timeAxisMinInterval('live', true),
    },
    yAxis: {
      type: 'value',
      axisLabel: { fontSize: 10, color: p.axisLabel },
      splitLine: { lineStyle: { color: p.gridLine } },
    },
    series: [
      {
        name: NET_SERIES[0],
        type: 'line',
        smooth: true,
        showSymbol: false,
        ...lineColors(1.5, netColors(p)[0]),
        data: [],
      },
      {
        name: NET_SERIES[1],
        type: 'line',
        smooth: true,
        showSymbol: false,
        ...lineColors(1.5, netColors(p)[1]),
        data: [],
      },
    ],
  }
}

/**
 * 网络图增量 option。
 *
 * 保留 3 位小数（`+(…).toFixed(3)`）：MB/s 常见量级在 0.001–1 之间，
 * 两位小数会把轻载机器的曲线压成一条 0 线；`+` 还原为数字避免 ECharts 把
 * 字符串当类目值。
 */
export function netUpdateOption(history: readonly TrendSample[]): EChartsCoreOption {
  return {
    series: [
      { data: history.map((s) => [toMillis(s.timestamp), +(s.network_rx_mbps ?? 0).toFixed(3)]) },
      { data: history.map((s) => [toMillis(s.timestamp), +(s.network_tx_mbps ?? 0).toFixed(3)]) },
    ],
  }
}

/** 负载仪表盘首屏 option（0–10 三色弧线） */
export function loadInitialOption(p: ChartPalette): EChartsCoreOption {
  return {
    tooltip: { trigger: 'item', ...tooltipStyle(p) },
    series: [
      {
        type: 'gauge',
        startAngle: 210,
        endAngle: -30,
        min: 0,
        max: GAUGE_MAX,
        radius: '95%',
        axisLine: {
          lineStyle: {
            width: 8,
            color: gaugeBands(p),
          },
        },
        pointer: { itemStyle: { color: 'auto' }, length: '55%', width: 4 },
        axisTick: { distance: -8, length: 4, lineStyle: { color: p.text, width: 1 } },
        splitLine: { distance: -8, length: 8, lineStyle: { color: p.text, width: 2 } },
        axisLabel: { color: p.axisLabel, distance: 14, fontSize: 9 },
        detail: {
          valueAnimation: true,
          formatter: '{value}',
          color: p.axisLabel,
          fontSize: 16,
          offsetCenter: [0, '55%'],
        },
        data: [{ value: 0 }],
      },
    ],
  }
}

/** 负载仪表盘增量 option（单值；两位小数与指标卡的负载读数同口径） */
export function loadUpdateOption(snapshot: MetricsSnapshot): EChartsCoreOption {
  return { series: [{ data: [{ value: +snapshot.load_one.toFixed(2) }] }] }
}

/**
 * 趋势图/网络图的主题补丁（主题切换时以**合并**方式重写轴与图例配色，不动 series）。
 *
 * 两图同构，共用一份补丁；刻意**不**整份重写 option——重建会把已积累的数据点
 * 清空（用户切主题就丢历史曲线），这是原实现的取舍，本函数保持。
 */
export function axisThemePatch(p: ChartPalette): EChartsCoreOption {
  return {
    legend: { textStyle: { color: p.axisLabel } },
    xAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
    yAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
  }
}

/**
 * 趋势图主题补丁：轴 / 图例 / tooltip + 三条线的配色。
 *
 * 系列补丁带 `name` 且**不含 `data`**——ECharts 按名称合并，已积累的曲线原样保留；
 * 此前补丁不动 series，切换品牌预设后曲线仍是旧品牌色。
 */
export function trendThemePatch(p: ChartPalette): EChartsCoreOption {
  return {
    ...axisThemePatch(p),
    tooltip: tooltipStyle(p),
    series: TREND_SERIES.map((name, i) => ({ name, ...lineColors(2, trendColors(p)[i]) })),
  }
}

/** 网络图主题补丁（同趋势图，RX / TX 两条线） */
export function netThemePatch(p: ChartPalette): EChartsCoreOption {
  return {
    ...axisThemePatch(p),
    tooltip: tooltipStyle(p),
    series: NET_SERIES.map((name, i) => ({ name, ...lineColors(1.5, netColors(p)[i]) })),
  }
}

/** 负载仪表盘的主题补丁（三色弧线、刻度、轴标签、中心数值、tooltip；不含 data） */
export function loadThemePatch(p: ChartPalette): EChartsCoreOption {
  return {
    tooltip: tooltipStyle(p),
    series: [
      {
        axisLine: { lineStyle: { color: gaugeBands(p) } },
        axisTick: { lineStyle: { color: p.text } },
        splitLine: { lineStyle: { color: p.text } },
        axisLabel: { color: p.axisLabel },
        detail: { color: p.axisLabel },
      },
    ],
  }
}
