import { describe, it, expect, beforeEach } from 'vitest'
import {
  trendInitialOption,
  trendUpdateOption,
  netInitialOption,
  netUpdateOption,
  loadInitialOption,
  loadUpdateOption,
  axisThemePatch,
  loadThemePatch,
  trendThemePatch,
  netThemePatch,
  readChartPalette,
  toMillis,
  timeAxisFormatter,
  timeAxisMinInterval,
  timeAxisPatch,
  historyToTrendSamples,
  TREND_SERIES,
  NET_SERIES,
  TREND_RANGES,
} from '@/utils/dashboardCharts'
import type { MetricsSnapshot } from '@/types'
import type { MetricsHistoryPoint } from '@/api/generated'

/**
 * T2-4 第五站的图表 option 纯函数单测。
 *
 * 这一段在拆分前**完全测不到**：option 结构内联在视图 setup 里，而 jsdom 没有
 * 画布，ECharts 实例在单测中不会真的绘图。抽成纯函数后，可以逐字段断言
 * 「轴的类型与量程、系列的数量与配色、增量数据的映射与精度」——
 * 这些正是改错了也不会报错、只会让曲线画歪的地方。
 */

const palette = {
  axisLabel: '#aaa',
  gridLine: '#333',
  text: '#fff',
  brand: '#f60',
  success: '#0f0',
  info: '#0af',
  warning: '#fc0',
  danger: '#f00',
  surface: '#111',
}

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
  network_rx_mbps: 1.2345,
  network_tx_mbps: 0.5,
  ...over,
})

/**
 * ECharts option 是深度嵌套的松散类型，测试侧逐字段断言时必须收口。
 * 这是测试文件内唯一一处 `any`（eslint 侧只 warning），避免为 17 个字段
 * 各写一份 interface——那种"类型安全"只会让断言比被测代码更难维护。
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyOption = Record<string, any>

describe('dashboardCharts · 趋势图', () => {
  it('三条系列：CPU / Memory / Disk，名字与常量同源', () => {
    const opt = trendInitialOption(palette) as AnyOption
    expect((opt.series as AnyOption[]).map((s) => s.name)).toEqual([...TREND_SERIES])
    expect(opt.legend.data).toEqual([...TREND_SERIES])
  })

  it('Y 轴固定 0–100（百分比语义；不设 max 会让曲线随数据缩放）', () => {
    const opt = trendInitialOption(palette) as AnyOption
    expect(opt.yAxis.max).toBe(100)
    expect(opt.yAxis.type).toBe('value')
  })

  it('X 轴是时间轴（时间轴才能让稀疏点自动分布）', () => {
    const opt = trendInitialOption(palette) as AnyOption
    expect(opt.xAxis.type).toBe('time')
  })

  it('配色取自调色板：CPU 品牌色 / 内存 info / 磁盘 success', () => {
    const opt = trendInitialOption(palette) as AnyOption
    const colors = (opt.series as AnyOption[]).map((s) => s.lineStyle.color)
    expect(colors).toEqual([palette.brand, palette.info, palette.success])
  })

  it('增量 option：三条系列各自的字段映射（CPU% / 内存% / 磁盘%），点为 [ms, 值] 对', () => {
    const history = [
      snap({
        timestamp: 1_700_000_000_000,
        cpu_usage: 10,
        memory_usage_percent: 20,
        disk_usage_percent: 30,
      }),
      snap({
        timestamp: 1_700_000_001_000,
        cpu_usage: 11,
        memory_usage_percent: 21,
        disk_usage_percent: 31,
      }),
    ]
    const opt = trendUpdateOption(history) as AnyOption
    expect((opt.series as AnyOption[]).map((s) => s.data)).toEqual([
      [
        [1_700_000_000_000, 10],
        [1_700_000_001_000, 11],
      ],
      [
        [1_700_000_000_000, 20],
        [1_700_000_001_000, 21],
      ],
      [
        [1_700_000_000_000, 30],
        [1_700_000_001_000, 31],
      ],
    ])
    // time 轴忽略 xAxis.data：时间必须编码在点里，否则刻度按索引、tooltip 显示 1970
    expect(opt.xAxis).toBeUndefined()
  })
})

describe('dashboardCharts · 时间轴（S-16）', () => {
  it('toMillis：WS 快照的 ms 时间戳原样，S-07 桶起点（秒）×1000', () => {
    expect(toMillis(1_700_000_000_000)).toBe(1_700_000_000_000)
    expect(toMillis(1_700_000_000)).toBe(1_700_000_000_000)
    expect(toMillis(0)).toBe(0)
  })

  it('刻度格式按范围自适应：实时到秒、小时级到分、7 天带日期；窄图去掉秒', () => {
    expect(timeAxisFormatter('live')).toBe('{HH}:{mm}:{ss}')
    expect(timeAxisFormatter('1h')).toBe('{HH}:{mm}')
    expect(timeAxisFormatter('24h')).toBe('{HH}:{mm}')
    expect(timeAxisFormatter('7d')).toBe('{MM}-{dd} {HH}:{mm}')
    expect(timeAxisFormatter('live', true)).toBe('{HH}:{mm}')
    expect(timeAxisFormatter('7d', true)).toBe('{MM}-{dd}')
  })

  it('最小刻度间隔随范围放大且单调递增；窄图翻倍', () => {
    const intervals = TREND_RANGES.map((r) => timeAxisMinInterval(r))
    expect(intervals).toEqual([5_000, 5 * 60_000, 60 * 60_000, 12 * 60 * 60_000])
    expect(timeAxisMinInterval('1h', true)).toBe(10 * 60_000)
  })

  it('轴补丁只含 xAxis（合并模式应用，不能带 series 否则覆盖曲线）', () => {
    const patch = timeAxisPatch('24h') as AnyOption
    expect(Object.keys(patch)).toEqual(['xAxis'])
    expect(patch.xAxis.axisLabel.formatter).toBe('{HH}:{mm}')
    expect(patch.xAxis.minInterval).toBe(60 * 60_000)
  })

  it('历史点 → 曲线样本：均值口径、缺失字段回落 0、时间戳转 ms', () => {
    const point: MetricsHistoryPoint = {
      bucket_start: 1_700_000_000,
      samples: 60,
      cpu_avg: 12.5,
      cpu_max: 40,
      memory_avg: 61.2,
      memory_max: 70,
      disk_avg: 44,
      load_one_avg: 0.8,
      net_rx_avg_mbps: 1.25,
      net_tx_avg_mbps: 0.5,
    }
    const [s] = historyToTrendSamples([point])
    expect(s).toEqual({
      timestamp: 1_700_000_000_000,
      cpu_usage: 12.5,
      memory_usage_percent: 61.2,
      disk_usage_percent: 44,
      network_rx_mbps: 1.25,
      network_tx_mbps: 0.5,
    })
    const [empty] = historyToTrendSamples([
      {
        ...point,
        cpu_avg: null,
        memory_avg: null,
        disk_avg: null,
        net_rx_avg_mbps: null,
        net_tx_avg_mbps: null,
      } as unknown as MetricsHistoryPoint,
    ])
    expect([empty.cpu_usage, empty.memory_usage_percent, empty.disk_usage_percent]).toEqual([
      0, 0, 0,
    ])
    expect([empty.network_rx_mbps, empty.network_tx_mbps]).toEqual([0, 0])
    expect(historyToTrendSamples([])).toEqual([])
  })
})

describe('dashboardCharts · 网络图', () => {
  it('两条系列：RX / TX', () => {
    const opt = netInitialOption(palette) as AnyOption
    expect((opt.series as AnyOption[]).map((s) => s.name)).toEqual([...NET_SERIES])
  })

  it('线宽 1.5（比趋势图细：同屏双曲线，主次要分明）', () => {
    const opt = netInitialOption(palette) as AnyOption
    expect((opt.series as AnyOption[]).map((s) => s.lineStyle.width)).toEqual([1.5, 1.5])
  })

  it('增量保留三位小数（两位会把轻载曲线压成 0 线）', () => {
    const history = [
      snap({ timestamp: 1_700_000_000_000, network_rx_mbps: 0.0012, network_tx_mbps: 0.9999 }),
    ]
    const opt = netUpdateOption(history) as AnyOption
    expect((opt.series as AnyOption[]).map((s) => s.data)).toEqual([
      [[1_700_000_000_000, 0.001]],
      [[1_700_000_000_000, 1]],
    ])
  })

  it('mbps 缺失回落 0（老库/字段缺失不产生 NaN 点）', () => {
    const history = [
      snap({
        timestamp: 1_700_000_000_000,
        network_rx_mbps: undefined as unknown as number,
        network_tx_mbps: undefined as unknown as number,
      }),
    ]
    const opt = netUpdateOption(history) as AnyOption
    expect((opt.series as AnyOption[])[0].data).toEqual([[1_700_000_000_000, 0]])
    expect((opt.series as AnyOption[])[1].data).toEqual([[1_700_000_000_000, 0]])
  })
})

describe('dashboardCharts · 负载仪表盘', () => {
  it('量程 0–10 与三色弧线（30% 绿 / 70% 黄 / 100% 红）', () => {
    const opt = loadInitialOption(palette) as AnyOption
    const gauge = (opt.series as AnyOption[])[0]
    expect(gauge.type).toBe('gauge')
    expect(gauge.min).toBe(0)
    expect(gauge.max).toBe(10)
    expect(gauge.axisLine.lineStyle.color).toEqual([
      [0.3, palette.success],
      [0.7, palette.warning],
      [1, palette.danger],
    ])
  })

  it('初始值为 0（未连接时仪表盘不显示上一次的残留值）', () => {
    const opt = loadInitialOption(palette) as AnyOption
    expect((opt.series as AnyOption[])[0].data).toEqual([{ value: 0 }])
  })

  it('增量取 load_one 且两位小数（与卡片读数同口径）', () => {
    const opt = loadUpdateOption(snap({ load_one: 3.456 })) as AnyOption
    expect((opt.series as AnyOption[])[0].data).toEqual([{ value: 3.46 }])
  })
})

describe('dashboardCharts · 主题补丁（不整份重建）', () => {
  it('折线图补丁只含轴与图例配色，不含 series.data（重建会丢历史曲线）', () => {
    const patch = axisThemePatch(palette) as AnyOption
    expect(patch.series).toBeUndefined()
    expect(patch.legend.textStyle.color).toBe(palette.axisLabel)
    expect(patch.xAxis.axisLabel.color).toBe(palette.axisLabel)
    expect(patch.yAxis.splitLine.lineStyle.color).toBe(palette.gridLine)
  })

  it('负载补丁：三色弧线 / 刻度 / 轴标签 / 中心数值随主题，不含数据', () => {
    const patch = loadThemePatch(palette) as AnyOption
    const series = (patch.series as AnyOption[])[0]
    expect(series.axisLabel.color).toBe(palette.axisLabel)
    expect(series.detail.color).toBe(palette.axisLabel)
    expect(series.axisLine.lineStyle.color).toEqual([
      [0.3, palette.success],
      [0.7, palette.warning],
      [1, palette.danger],
    ])
    expect(series.axisTick.lineStyle.color).toBe(palette.text)
    expect(series.data).toBeUndefined()
    expect(patch.tooltip.backgroundColor).toBe(palette.surface)
  })

  it('趋势 / 网络补丁按名称重写系列配色，但绝不携带 data（切主题不丢曲线）', () => {
    const trend = trendThemePatch(palette) as AnyOption
    expect((trend.series as AnyOption[]).map((s) => s.name)).toEqual([...TREND_SERIES])
    expect((trend.series as AnyOption[]).map((s) => s.lineStyle.color)).toEqual([
      palette.brand,
      palette.info,
      palette.success,
    ])
    expect((trend.series as AnyOption[]).every((s) => s.data === undefined)).toBe(true)
    expect(trend.xAxis.axisLabel.color).toBe(palette.axisLabel)
    expect(trend.tooltip.textStyle.color).toBe(palette.text)

    const net = netThemePatch(palette) as AnyOption
    expect((net.series as AnyOption[]).map((s) => [s.name, s.itemStyle.color])).toEqual([
      [NET_SERIES[0], palette.info],
      [NET_SERIES[1], palette.success],
    ])
    expect((net.series as AnyOption[]).every((s) => s.data === undefined)).toBe(true)
  })

  it('首屏 option 与主题补丁同源：tooltip 取浮层令牌', () => {
    for (const opt of [
      trendInitialOption(palette),
      netInitialOption(palette),
      loadInitialOption(palette),
    ]) {
      const tip = (opt as AnyOption).tooltip
      expect(tip.backgroundColor).toBe(palette.surface)
      expect(tip.borderColor).toBe(palette.gridLine)
    }
  })
})

describe('dashboardCharts · readChartPalette', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('style')
  })

  it('从 CSS 变量读取令牌（未定义时字段为 undefined，不写死颜色）', () => {
    document.documentElement.style.setProperty('--fp-brand', '#123456')
    document.documentElement.style.setProperty('--fp-text-secondary', '#abcdef')
    const p = readChartPalette()
    expect(p.brand).toBe('#123456')
    expect(p.axisLabel).toBe('#abcdef')
    expect(p.danger).toBeUndefined()
  })

  it('axisLabel 两级回落：text-secondary → text-muted', () => {
    document.documentElement.style.setProperty('--fp-text-muted', '#777777')
    expect(readChartPalette().axisLabel).toBe('#777777')
  })

  it('gridLine 两级回落：border → border-strong', () => {
    document.documentElement.style.setProperty('--fp-border-strong', '#555555')
    expect(readChartPalette().gridLine).toBe('#555555')
  })
})
