import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import NetworkListPanel from '@/components/docker/NetworkListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的网络列表契约（原为 DockerView 内联表格）。
 *
 * 重点：**已连接容器列**的两种呈现（标签带 IP / 破折号占位）与列宽冻结语义。
 * 该列是「网络为什么连不上」这类问题的第一现场——把无连接渲染成空白会让
 * 用户误判为加载失败，故占位符 `—` 是契约的一部分。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = {
  name: 'FpTable',
  props: {
    rows: { type: Array, default: () => [] },
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
  docker: {
    name: '名称',
    driver: '驱动',
    scope: '范围',
    connectedContainers: '已连接容器',
    actions: '操作',
    connect: '连接',
    disconnect: '断开',
    remove: '删除',
    prune: '清理',
    createNetwork: '创建网络',
  },
  common: { noData: '暂无数据' },
}

const network = (over: Record<string, unknown> = {}) => ({
  id: 'net1',
  name: 'app-net',
  driver: 'bridge',
  scope: 'local',
  internal: false,
  attachable: true,
  created: null,
  containers: [],
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(NetworkListPanel, {
    props: { rows: [network()], ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

describe('NetworkListPanel · 列与连接列呈现', () => {
  it('声明五列（名称/驱动/范围/已连接容器/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['名称', '驱动', '范围', '已连接容器', '操作'])
  })

  it('有连接：每个容器一个标签，带 IPv4 时括号跟随', () => {
    const w = mountPanel({
      rows: [
        network({
          containers: [
            { name: 'web', ipv4_address: '172.20.0.2', ipv6_address: null },
            { name: 'db', ipv4_address: null, ipv6_address: null },
          ],
        }),
      ],
    })
    const tags = w.findAll('[data-col="3"] .fp-tag')
    expect(tags.map((t) => t.text().replace(/\s+/g, ' '))).toEqual(['web (172.20.0.2)', 'db'])
  })

  it('无连接：渲染破折号占位（不是空白——空白会被误读为「没加载出来」）', () => {
    const w = mountPanel({ rows: [network({ containers: [] })] })
    expect(w.find('[data-col="3"]').text()).toBe('—')
    expect(w.find('[data-col="3"] .fp-tag').exists()).toBe(false)
  })

  it('containers 为 null（后端未返回该字段）同样走占位分支，不抛错', () => {
    const w = mountPanel({ rows: [network({ containers: null })] })
    expect(w.find('[data-col="3"]').text()).toBe('—')
  })
})

describe('NetworkListPanel · 工具栏与操作上抛', () => {
  it('工具栏两个入口：创建网络 + 清理', () => {
    const w = mountPanel()
    expect(w.findAll('.toolbar button').map((b) => b.text())).toEqual(['创建网络', '清理'])
  })

  it('创建与清理各自上抛（无载荷）', async () => {
    const w = mountPanel()
    await w.findAll('.toolbar button')[0].trigger('click')
    await w.findAll('.toolbar button')[1].trigger('click')
    expect(w.emitted('create')).toBeTruthy()
    expect(w.emitted('prune')).toBeTruthy()
  })

  it('三个行内操作各上抛对应事件并携带整行', async () => {
    const w = mountPanel()
    const btns = w.findAll('.row-actions button')
    expect(btns.map((b) => b.text())).toEqual(['连接', '断开', '删除'])

    for (const b of btns) await b.trigger('click')
    for (const name of ['connect', 'disconnect', 'remove']) {
      expect(w.emitted(name)?.[0][0], `未上抛 ${name}`).toMatchObject({ id: 'net1' })
    }
  })
})
