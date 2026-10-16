import { describe, it, expect } from 'vitest'
import { mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import NodeListPanel from '@/components/node/NodeListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站拆出的节点列表契约（原为 NodesView 内联表格）。
 *
 * **表格替身（TableStub）**：与 ProjectListPanel.spec 同一取舍——虚拟滚动依赖
 * 真实布局尺寸，jsdom 没有排版引擎，真表一行都不渲染。替身按 DataTable 的公开
 * 契约（`rows` + 各列 `body` 插槽逐行求值）复刻，单元格逻辑得到真实覆盖。
 *
 * 本表的重点在**三处指标展示口径**（它们改错了不报错，只会显示错的数字）：
 * ① 无指标时渲染 `—`（不是 `0%`）；
 * ② 指标条的宽度与阈值取色；
 * ③ 心跳缺失同样 `—`。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = {
  name: 'FpTable',
  props: {
    rows: { type: Array, default: () => [] },
    loading: { type: Boolean, default: false },
    emptyText: { type: String, default: '' },
  },
  setup(
    props: { rows: Record<string, unknown>[]; emptyText: string },
    { slots }: { slots: { default?: () => unknown } },
  ) {
    return () => {
      const columns = (slots.default?.() ?? []) as ColumnVNode[]
      if (!props.rows.length) return h('div', { class: 'stub-empty' }, props.emptyText)
      return h(
        'div',
        { class: 'stub-table' },
        props.rows.map((row, index) =>
          h(
            'div',
            { class: 'stub-row', 'data-p-index': index },
            columns.map((col, colIndex) => {
              const field = col.props?.field as string | undefined
              const body = col.children?.body
              return h(
                'div',
                {
                  class: 'stub-cell',
                  'data-col': colIndex,
                  'data-header': String(col.props?.header ?? ''),
                },
                body ? (body({ data: row }) as never) : String(field ? (row[field] ?? '') : ''),
              )
            }),
          ),
        ),
      )
    }
  },
}

const messages = {
  node: {
    id: 'ID',
    name: '名称',
    hostname: '主机名',
    ip: 'IP 地址',
    status: '状态',
    metrics: '实时指标',
    lastHeartbeat: '最近心跳',
    liveHint: '节点状态每 10 秒自动刷新（基于 Agent 心跳）',
    remoteExec: '远程命令',
    remoteFiles: '远程文件',
    monitor: '监控',
    terminal: '远程终端',
    edge: 'Edge',
    edgeHint: '经 Agent 出站反向通道连接',
  },
  dashboard: { online: '在线', offline: '离线' },
  common: { noData: '暂无数据', operation: '操作', edit: '编辑', delete: '删除' },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => VueWrapper<Record<string, unknown>>

function mountPanel(rows: Record<string, unknown>[], props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  return mountLoose(NodeListPanel, {
    props: { rows, ...props },
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { FpTable: TableStub, FpColumn: false, FpButton: false, FpTag: false },
    },
  })
}

const node = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'db-1',
  hostname: 'db-1.internal',
  ip_address: '10.0.0.1',
  status: 'online',
  created_at: '',
  online: true,
  metrics: { cpu_usage: 12.34, memory_usage_percent: 45.67, disk_usage_percent: 78.9 },
  last_heartbeat_at: '2026-09-15 10:00:00',
  ...over,
})

/** 取第 index 列的一整列单元格（DOMWrapper 便于逐格断言） */
function columnCells(w: VueWrapper, index: number): DOMWrapper<Element>[] {
  return w.findAll(`.stub-cell[data-col="${index}"]`)
}

/** 取第 index 列的一整列文本 */
function columnText(w: VueWrapper, index: number): string {
  return columnCells(w, index)
    .map((c) => c.text())
    .join('|')
}

describe('NodeListPanel · 列与状态', () => {
  it('渲染八列（含可见性提示条）', () => {
    const w = mountPanel([node()])
    expect(w.text()).toContain('节点状态每 10 秒自动刷新')
    const headers = w.findAll('.stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toContain('实时指标')
    expect(headers).toContain('最近心跳')
    expect(headers).toContain('操作')
  })

  it('在线/离线按 online 字段映射标签语义（不是 status 字段）', () => {
    const w = mountPanel([
      node({ id: 1, status: 'online', online: true }),
      node({ id: 2, status: 'online', online: false }),
    ])
    const text = columnText(w, 4)
    expect(text).toContain('在线')
    expect(text).toContain('离线')
  })

  it('空列表渲染空态文案', () => {
    const w = mountPanel([])
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })
})

describe('NodeListPanel · 指标展示口径', () => {
  it('【关键】无指标渲染 —（不是 0%）', () => {
    const w = mountPanel([node({ metrics: null })])
    expect(columnText(w, 5)).toBe('—')
  })

  it('【关键】有指标渲染摘要文本 + 进度条（宽度取整、阈值取色）', () => {
    const w = mountPanel([node()])
    const cell = w.find('.stub-cell[data-col="5"]')
    expect(cell.text()).toContain('CPU 12.3% · MEM 45.7% · DISK 78.9%')

    const bar = cell.find('.metrics-bar-fill').element as HTMLElement
    // 进度条看的是 **CPU**（12.34 → 12%、绿），不是磁盘
    expect(bar.style.width).toBe('12%')
    expect(bar.style.background).toBe('var(--fp-success)')
  })

  it('【关键】进度条只看 CPU：磁盘 92% 也不会让 CPU 条变红', () => {
    const w = mountPanel([
      node({ metrics: { cpu_usage: 5, memory_usage_percent: 1, disk_usage_percent: 92 } }),
    ])
    const bar = w.find('.metrics-bar-fill').element as HTMLElement
    expect(bar.style.width).toBe('5%')
    expect(bar.style.background).toBe('var(--fp-success)')
    // 但摘要里磁盘读数照常是 92%
    expect(w.find('.stub-cell[data-col="5"]').text()).toContain('DISK 92.0%')
  })

  it('高负载转红（>80），低负载为绿', () => {
    const high = mountPanel([node({ metrics: { cpu_usage: 92, memory_usage_percent: 1, disk_usage_percent: 1 } })])
    expect((high.find('.metrics-bar-fill').element as HTMLElement).style.background).toBe(
      'var(--fp-danger)',
    )

    const low = mountPanel([node({ metrics: { cpu_usage: 10, memory_usage_percent: 1, disk_usage_percent: 1 } })])
    expect((low.find('.metrics-bar-fill').element as HTMLElement).style.background).toBe(
      'var(--fp-success)',
    )
  })

  it('心跳缺失渲染 —', () => {
    const w = mountPanel([node({ last_heartbeat_at: null })])
    expect(columnText(w, 6)).toBe('—')
  })
})

describe('NodeListPanel · 行操作', () => {
  it('【关键】五类行操作齐备且各自上抛整行', async () => {
    const w = mountPanel([node()])
    const buttons = w.findAll('.stub-cell[data-col="7"] button')
    const labels = buttons.map((b) => b.text().trim())

    expect(labels).toEqual(['监控', '编辑', '远程命令', '远程文件', '删除'])

    const events = ['monitor', 'edit', 'exec', 'files', 'remove'] as const
    for (const [i, event] of events.entries()) {
      await buttons[i].trigger('click')
      const emitted = w.emitted(event)
      expect(emitted, `${event} 未上抛`).toBeTruthy()
      expect((emitted as unknown[][])[0][0]).toMatchObject({ id: 1, name: 'db-1' })
    }
  })
})

describe('NodeListPanel · Agent Edge', () => {
  it('【关键】Edge 在线显示徽标；未连接 / 未知不显示', () => {
    const w = mountPanel([
      node({ id: 1, edge: { connected: true, terminal: false, version: '1', connected_at: null } }),
      node({ id: 2, name: 'db-2', edge: null }),
    ])
    const cells = columnCells(w, 4)
    expect(cells[0].text()).toContain('Edge')
    expect(cells[1].text()).not.toContain('Edge')
  })

  it('【关键】「远程终端」仅在 Edge 在线且 Agent 开启终端时出现，点击上抛整行', async () => {
    const edge = (terminal: boolean) => ({ connected: true, terminal, version: '1', connected_at: null })
    const w = mountPanel([node({ id: 1, edge: edge(true) }), node({ id: 2, name: 'db-2', edge: edge(false) })])
    const rows = columnCells(w, 7)
    expect(rows[0].text()).toContain('远程终端')
    expect(rows[1].text()).not.toContain('远程终端')
    const btn = rows[0].findAll('button').find((b) => b.text().trim() === '远程终端')
    await btn?.trigger('click')
    expect((w.emitted('terminal') as unknown[][])[0][0]).toMatchObject({ id: 1 })
  })
})

describe('NodeListPanel · 分页', () => {
  it('total <= pageSize 不渲染分页条', () => {
    const w = mountPanel([node()], { total: 1, pageSize: 20 })
    expect(w.findComponent({ name: 'FpPagination' }).exists()).toBe(false)
  })

  it('【关键】total > pageSize 渲染分页条，翻页上抛偏移量', async () => {
    const w = mountPanel([node()], { total: 100, pageSize: 20, first: 0 })
    const pag = w.findComponent({ name: 'FpPagination' })
    expect(pag.exists()).toBe(true)

    pag.vm.$emit('update:first', 40)
    await w.vm.$nextTick()
    expect(w.emitted('update:first')?.[0]).toEqual([40])
  })
})
