import { describe, it, expect } from 'vitest'
import {
  containerStatusType,
  filterContainers,
  logTailOptions,
  driverOptions,
  connectContainerOptions,
  NETWORK_DRIVERS,
  VOLUME_DRIVERS,
} from '@/utils/dockerFormat'
import type { DockerContainer } from '@/types'

/**
 * T2-4 第三站拆出的 Docker 公共纯函数契约（原为 DockerView 内联实现）。
 *
 * 这里锁的不是「函数能跑」，而是四条**有产品含义的映射规则**：
 * ① 未知容器状态不猜成「运行中」；② 搜索覆盖名称/ID/镜像三字段；
 * ③ 驱动候选来自白名单常量（翻译会与官方文档对不上）；
 * ④ 连接下拉在容器名为空时回落 ID（不渲染空标签）。
 */

const container = (over: Partial<DockerContainer> = {}): DockerContainer => ({
  id: 'a1b2c3d4e5f6',
  image: 'nginx:latest',
  name: 'web',
  status: 'running',
  node_id: 1,
  created_at: '2026-01-01T00:00:00Z',
  ...over,
})

describe('dockerFormat · 容器状态映射', () => {
  it('running → success（绿）', () => {
    expect(containerStatusType('running')).toBe('success')
  })

  it('paused → warning（黄）', () => {
    expect(containerStatusType('paused')).toBe('warning')
  })

  it('exited / dead → danger（红）', () => {
    expect(containerStatusType('exited')).toBe('danger')
    expect(containerStatusType('dead')).toBe('danger')
  })

  it('未知状态回落 info，**不回落 success**——把 created/restarting 显示成「运行中」是最误导运维的映射错误', () => {
    expect(containerStatusType('created')).toBe('info')
    expect(containerStatusType('restarting')).toBe('info')
    expect(containerStatusType('removing')).toBe('info')
    expect(containerStatusType('')).toBe('info')
  })
})

describe('dockerFormat · 容器搜索', () => {
  const rows = [
    container({ id: 'aaa111', name: 'web', image: 'nginx:latest' }),
    container({ id: 'bbb222', name: 'db', image: 'postgres:16' }),
    container({ id: 'ccc333', name: 'cache', image: 'redis:alpine' }),
  ]

  it('空查询与纯空白查询返回原数组（不复制、不过滤）', () => {
    expect(filterContainers(rows, '')).toBe(rows)
    expect(filterContainers(rows, '   ')).toBe(rows)
  })

  it('按名称匹配且大小写不敏感', () => {
    expect(filterContainers(rows, 'WEB').map((c) => c.name)).toEqual(['web'])
  })

  it('按镜像匹配（含冒号标签）', () => {
    expect(filterContainers(rows, 'postgres:16').map((c) => c.name)).toEqual(['db'])
  })

  it('按 ID 匹配', () => {
    expect(filterContainers(rows, 'ccc').map((c) => c.name)).toEqual(['cache'])
  })

  it('无命中返回空数组（而不是全量——空数组驱动表格空态）', () => {
    expect(filterContainers(rows, 'not-there')).toEqual([])
  })
})

describe('dockerFormat · 下拉候选与常量', () => {
  it('日志行数三档 + 「全部」哨兵 9999，末项标签不带数字', () => {
    const opts = logTailOptions((k) => k)
    expect(opts.map((o) => o.value)).toEqual([50, 100, 500, 9999])
    expect(opts[0].label).toBe('50 docker.tailLines')
    expect(opts[3].label).toBe('docker.tailLines')
  })

  it('驱动候选来自白名单常量（网络 4 项 / 卷 2 项，label=value 不翻译）', () => {
    expect(driverOptions(NETWORK_DRIVERS).map((o) => o.value)).toEqual([
      'bridge',
      'host',
      'overlay',
      'macvlan',
    ])
    expect(driverOptions(VOLUME_DRIVERS).map((o) => o.value)).toEqual(['local', 'nfs'])
    for (const o of driverOptions(NETWORK_DRIVERS)) expect(o.label).toBe(o.value)
  })

  it('连接容器候选：名称为标签、ID 为值', () => {
    const opts = connectContainerOptions([container(), container({ id: 'x9', name: 'db' })])
    expect(opts).toEqual([
      { label: 'web', value: 'a1b2c3d4e5f6' },
      { label: 'db', value: 'x9' },
    ])
  })

  it('容器名为空时下拉标签回落 ID（不出现空标签）', () => {
    const opts = connectContainerOptions([container({ name: '', id: 'deadbeef' })])
    expect(opts).toEqual([{ label: 'deadbeef', value: 'deadbeef' }])
  })
})
