import { describe, it, expect } from 'vitest'
import {
  usageColor,
  formatMbps,
  buildStatCards,
  sparkPath,
  statTone,
  LOAD_GAUGE_MAX,
} from '@/utils/dashboardFormat'
import type { MetricsSnapshot } from '@/types'

/**
 * T2-4 第五站的纯函数单测（`DashboardView` → `utils/dashboardFormat.ts`）。
 *
 * 重点看住三处「改了不报错、只是显示差一点」的口径：
 * ① 使用率配色的边界（80 仍黄、50 仍绿——阈值语义是 `>` 不是 `>=`）；
 * ② 负载进度条的封顶（负载可以超过 10，不封顶会画出越轨的进度条）；
 * ③ 四种单位换算各自的小数位（内存 MB→GB、磁盘 GB 直读，均 1 位）。
 */

const snap = (over: Partial<MetricsSnapshot> = {}): MetricsSnapshot => ({
  timestamp: 1700000000,
  cpu_usage: 12.34,
  cpu_cores: 8,
  memory_usage_percent: 63.75,
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

/** 测试用 t：把 key 原样返回，便于断言「取的是哪个 key」 */
const t = (key: string) => key

describe('dashboardFormat · usageColor 阈值边界', () => {
  it('>80 红 / 50<x<=80 黄 / <=50 绿', () => {
    expect(usageColor(95)).toBe('var(--fp-danger)')
    expect(usageColor(51)).toBe('var(--fp-warning)')
    expect(usageColor(10)).toBe('var(--fp-success)')
  })

  it('边界值 80 仍为黄、50 仍为绿（阈值是 > 不是 >=）', () => {
    expect(usageColor(80)).toBe('var(--fp-warning)')
    expect(usageColor(50)).toBe('var(--fp-success)')
  })

  it('0 与 100 分别落在两端（不抛错、不越界）', () => {
    expect(usageColor(0)).toBe('var(--fp-success)')
    expect(usageColor(100)).toBe('var(--fp-danger)')
  })
})

describe('dashboardFormat · formatMbps', () => {
  it('两位小数（与曲线三位精度刻意不同）', () => {
    expect(formatMbps(1.234)).toBe('1.23')
    expect(formatMbps(0.5)).toBe('0.50')
  })

  it('null / undefined 回落 0.00（不渲染 NaN）', () => {
    expect(formatMbps(null)).toBe('0.00')
    expect(formatMbps(undefined)).toBe('0.00')
  })
})

describe('dashboardFormat · buildStatCards 四卡拼装', () => {
  it('四张卡的 key 与跳转目标一一对应', () => {
    const cards = buildStatCards(snap(), t)
    expect(cards.map((c) => c.key)).toEqual(['cpu', 'mem', 'disk', 'load'])
    expect(cards.map((c) => c.to)).toEqual(['/nodes', '/nodes', '/files', '/health'])
  })

  it('label 取自 i18n key（CPU / 内存 / 磁盘 / 系统负载）', () => {
    const cards = buildStatCards(snap(), t)
    expect(cards.map((c) => c.label)).toEqual([
      'dashboard.cpu',
      'dashboard.memory',
      'dashboard.disk',
      'dashboard.load',
    ])
  })

  it('百分比卡：value 一位小数 + percent 四舍五入', () => {
    const [cpu] = buildStatCards(snap({ cpu_usage: 12.34 }), t)
    expect(cpu.value).toBe('12.3%')
    expect(cpu.percent).toBe(12)
    const [, mem] = buildStatCards(snap({ memory_usage_percent: 63.75 }), t)
    expect(mem.value).toBe('63.8%') // toFixed 四舍五入（63.75 → 63.8）
    expect(mem.percent).toBe(64)
  })

  it('内存 detail：MB→GB 两位读数（各自一位小数）', () => {
    const [, mem] = buildStatCards(snap({ memory_used_mb: 8192, memory_total_mb: 16384 }), t)
    expect(mem.detail).toBe('8.0 / 16.0 GB')
  })

  it('磁盘 detail：GB 直读（不做二次换算）', () => {
    const [, , disk] = buildStatCards(snap({ disk_used_gb: 44.25, disk_total_gb: 100 }), t)
    expect(disk.detail).toBe('44.3 / 100.0 GB')
  })

  it('CPU detail 带核数（含 `cores` 后缀，不翻译）', () => {
    const [cpu] = buildStatCards(snap({ cpu_cores: 16 }), t)
    expect(cpu.detail).toBe('16 cores')
  })

  it('负载卡：value 两位小数，detail 三值并列', () => {
    const [, , , load] = buildStatCards(snap({ load_one: 3.456, load_five: 2.5, load_fifteen: 1.25 }), t)
    expect(load.value).toBe('3.46')
    expect(load.detail).toBe('1m: 3.46 | 5m: 2.50 | 15m: 1.25')
  })

  it('负载进度条按 1/10 映射（LOAD_GAUGE_MAX）', () => {
    const [, , , load] = buildStatCards(snap({ load_one: 2.5 }), t)
    expect(load.percent).toBe(25)
    expect(LOAD_GAUGE_MAX).toBe(10)
  })

  it('负载超过 10 时进度条封顶 100（不越轨）', () => {
    const [, , , load] = buildStatCards(snap({ load_one: 34.5 }), t)
    expect(load.percent).toBe(100)
    // 数值本身不被截断——运维要知道负载到底超了多少
    expect(load.value).toBe('34.50')
  })

  it('配色：CPU/内存/磁盘按使用率，负载恒绿（负载无百分比语义）', () => {
    const cards = buildStatCards(
      snap({ cpu_usage: 95, memory_usage_percent: 60, disk_usage_percent: 10 }),
      t,
    )
    expect(cards.map((c) => c.color)).toEqual([
      'var(--fp-danger)',
      'var(--fp-warning)',
      'var(--fp-success)',
      'var(--fp-success)',
    ])
  })
})

describe('sparkPath · 迷你趋势线', () => {
  it('少于 2 点返回空串（调用方据此不渲染）', () => {
    expect(sparkPath([])).toBe('')
    expect(sparkPath([5])).toBe('')
  })

  it('归一化到 viewBox 100×28：最小值贴底(26)、最大值贴顶(2)、x 等分', () => {
    expect(sparkPath([0, 50, 100])).toBe('M0.0,26.0 L50.0,14.0 L100.0,2.0')
  })

  it('常数序列不除零：全部落在底线', () => {
    expect(sparkPath([7, 7, 7])).toBe('M0.0,26.0 L50.0,26.0 L100.0,26.0')
  })
})

describe('statTone · 三段色阶', () => {
  it('资源卡 60/85 阈值', () => {
    expect(statTone(59.9, 'cpu')).toBe('ok')
    expect(statTone(60, 'mem')).toBe('warn')
    expect(statTone(85, 'disk')).toBe('bad')
  })
  it('负载卡 50/80 阈值', () => {
    expect(statTone(49, 'load')).toBe('ok')
    expect(statTone(50, 'load')).toBe('warn')
    expect(statTone(80, 'load')).toBe('bad')
  })
})
