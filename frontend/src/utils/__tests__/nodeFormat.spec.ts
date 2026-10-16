import { describe, it, expect } from 'vitest'
import {
  buildNodeUpdateBody,
  cpuBarStyle,
  decodeBase64,
  encodeBase64,
  enrichNode,
  hasFieldErrors,
  initNodeForm,
  joinRemotePath,
  nodeFormFrom,
  nodeMetricsSummary,
  nodeStatusOptions,
  parentRemotePath,
  parseNodeMetrics,
  percentColor,
  validateNodeFields,
} from '@/utils/nodeFormat'
import { usageColor } from '@/utils/dashboardFormat'

/**
 * T2-4 第七站拆出的纯函数层（原为 NodesView 824 行里的内联实现）。
 *
 * 这层里有三处「改错了不报错、只会让用户看到错的数字」的口径，是本 spec 的重点：
 * 默认指标值补 0、CPU 条宽度取整而不夹取、路径上跳不越过根。
 */

const t = (key: string) => `t:${key}`

const metrics = (over: Record<string, unknown> = {}) => ({
  cpu_usage: 12.34,
  memory_usage_percent: 45.67,
  disk_usage_percent: 78.9,
  load_one: 0.5,
  ...over,
})

describe('percentColor · 阈值与同源', () => {
  it('>80 红 / >50 黄 / 其余绿', () => {
    expect(percentColor(81)).toBe('var(--fp-danger)')
    expect(percentColor(51)).toBe('var(--fp-warning)')
    expect(percentColor(50)).toBe('var(--fp-success)')
  })

  it('【关键】与仪表盘 usageColor 是**同一个函数**（不是抄的第二份）', () => {
    expect(percentColor).toBe(usageColor)
  })

  it('边界取 `>` 而非 `>=`：恰好 80 仍是黄、恰好 50 仍是绿', () => {
    expect(percentColor(80)).toBe('var(--fp-warning)')
    expect(percentColor(50)).toBe('var(--fp-success)')
  })
})

describe('parseNodeMetrics / enrichNode', () => {
  it('正常 JSON 解析成对象', () => {
    expect(parseNodeMetrics('{"cpu_usage":7}')).toEqual({ cpu_usage: 7 })
  })

  it('【关键】脏 JSON 回落到 null 而不是抛错（脏数据不该让整张表打不开）', () => {
    expect(parseNodeMetrics('{半截写入')).toBeNull()
    expect(parseNodeMetrics('')).toBeNull()
    expect(parseNodeMetrics(null)).toBeNull()
    expect(parseNodeMetrics(undefined)).toBeNull()
  })

  it('【关键】enrichNode 的 online 恒为 false（列表接口的 status 是「注册时填的期望值」）', () => {
    const enriched = enrichNode({
      id: 1,
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'online',
      created_at: '',
      metrics_json: JSON.stringify(metrics()),
    })
    // 即使行里写着 online，富化产物也不认——真实在线状态只由 /status 判定
    expect(enriched.online).toBe(false)
    expect(enriched.metrics).toEqual(metrics())
  })

  it('metrics_json 为 null 时指标为 null（模板据此渲染 —）', () => {
    const enriched = enrichNode({
      id: 1,
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'offline',
      created_at: '',
      metrics_json: null,
    })
    expect(enriched.metrics).toBeNull()
  })
})

describe('cpuBarStyle / nodeMetricsSummary', () => {
  it('宽度取四舍五入的整数百分比', () => {
    expect(cpuBarStyle(metrics({ cpu_usage: 12.34 })).width).toBe('12%')
    expect(cpuBarStyle(metrics({ cpu_usage: 12.5 })).width).toBe('13%')
  })

  it('【关键】超过 100 也不夹取（夹取会把「读数异常」伪装成「满格」）', () => {
    expect(cpuBarStyle(metrics({ cpu_usage: 137 })).width).toBe('137%')
  })

  it('缺值按 0 处理，取色为绿', () => {
    expect(cpuBarStyle(null)).toEqual({ width: '0%', background: 'var(--fp-success)' })
  })

  it('摘要三个维度一律一位小数', () => {
    expect(nodeMetricsSummary(metrics())).toBe('CPU 12.3% · MEM 45.7% · DISK 78.9%')
  })

  it('全部缺失时补 0（不渲染 undefined%）', () => {
    expect(nodeMetricsSummary({})).toBe('CPU 0.0% · MEM 0.0% · DISK 0.0%')
    expect(nodeMetricsSummary(null)).toBe('CPU 0.0% · MEM 0.0% · DISK 0.0%')
  })
})

describe('路径拼接与上跳', () => {
  it('根目录 `.` 下直接落名字，否则用 / 连接', () => {
    expect(joinRemotePath('.', 'app.log')).toBe('app.log')
    expect(joinRemotePath('/var/log', 'app.log')).toBe('/var/log/app.log')
  })

  it('【关键】上跳不越过根：`.` 之上仍是 `.`（不会拼出 .. 送进后端）', () => {
    expect(parentRemotePath('.')).toBe('.')
    expect(parentRemotePath('app.log')).toBe('.')
  })

  it('上跳去掉最后一段，并容忍前导斜杠与双斜杠', () => {
    expect(parentRemotePath('/var/log')).toBe('var')
    expect(parentRemotePath('var/log')).toBe('var')
    expect(parentRemotePath('a//b')).toBe('a')
  })
})

describe('base64 编解码', () => {
  it('UTF-8 往返：中文与 emoji 都不丢字节', () => {
    const text = '第一行\n第二行 🚀\t结束'
    expect(new TextDecoder().decode(decodeBase64(encodeBase64(text)))).toBe(text)
  })

  it('【关键】decodeBase64 只做字节还原（不做文本解码，二进制内容不被改写）', () => {
    // 0x00 0xFF 0x01 —— 若走 TextDecoder 会被替换成 U+FFFD
    expect(Array.from(decodeBase64('AP8B'))).toEqual([0, 255, 1])
  })
})

describe('表单：初值、回填、校验、更新体', () => {
  it('初值：status 默认 online', () => {
    expect(initNodeForm()).toEqual({ name: '', hostname: '', ip_address: '', status: 'online' })
  })

  it('【关键】编辑回填只取四个可编辑字段（Agent 上报的快照不回送）', () => {
    const form = nodeFormFrom({
      id: 3,
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'offline',
      created_at: '2026-01-01',
      metrics_json: '{"cpu_usage":99}',
      last_heartbeat_at: '2026-01-02',
    } as never)
    expect(form).toEqual({ name: 'n1', hostname: 'h1', ip_address: '10.0.0.1', status: 'offline' })
    expect(form).not.toHaveProperty('metrics_json')
  })

  it('校验：三项逐一给红字，全空时三条都错', () => {
    const errors = validateNodeFields(initNodeForm(), t)
    expect(errors).toEqual({
      name: 't:node.nameRequired',
      hostname: 't:node.hostnameRequired',
      ip_address: 't:node.ipRequired',
    })
    expect(hasFieldErrors(errors)).toBe(true)
  })

  it('【口径】不做格式校验：内网短名与主机名都通过', () => {
    const errors = validateNodeFields(
      { name: 'db-1', hostname: 'db', ip_address: 'db.internal', status: 'online' },
      t,
    )
    expect(errors).toEqual({ name: '', hostname: '', ip_address: '' })
    expect(hasFieldErrors(errors)).toBe(false)
  })

  it('更新体：只带四个可编辑字段 + created_at 占位', () => {
    const body = buildNodeUpdateBody(3, {
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'online',
    })
    expect(body).toEqual({
      id: 3,
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'online',
      created_at: '',
    })
    expect(body).not.toHaveProperty('metrics_json')
  })
})

describe('nodeStatusOptions', () => {
  it('两档：online / offline（label 按当前语言现算）', () => {
    expect(nodeStatusOptions(t)).toEqual([
      { label: 't:dashboard.online', value: 'online' },
      { label: 't:dashboard.offline', value: 'offline' },
    ])
  })
})
