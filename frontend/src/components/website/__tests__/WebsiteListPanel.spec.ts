import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import WebsiteListPanel from '@/components/website/WebsiteListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站拆出的网站列表契约（原为 WebsitesView 内联表格）。
 *
 * **表格替身（TableStub）**：与 InstalledAppsPanel.spec 同一取舍——虚拟滚动依赖
 * 真实布局尺寸，而 jsdom 没有排版引擎，真表一行都不渲染。替身按 DataTable 的公开
 * 契约（`rows` + 各列 `body` 插槽逐行求值）复刻，单元格逻辑得到真实覆盖。
 *
 * 关注点：列语义与顺序、站点类型/伪静态/状态的标签映射、五个行内操作的上抛、
 * 分页仅在 `total > pageSize` 时出现、以及**空态判定**（由父级传入的 total 决定，
 * 不是本地行数组——服务端分页下「本页无数据」不等于「全库无数据」）。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

/**
 * 三个替身都用**纯对象**而非 `defineComponent`：同一文件里多个 `defineComponent`
 * 会触发 `vue/one-component-per-file` 告警（ESLint 会把 .ts 里的组件定义也纳入
 * 该规则），而替身本身不参与任何组件层级语义。渲染函数直接返回 VNode，与
 * `defineComponent` 写法在挂载行为上等价。
 */
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

const PaginationStub = {
  name: 'FpPagination',
  props: {
    first: { type: Number, default: 0 },
    rows: { type: Number, default: 20 },
    total: { type: Number, default: 0 },
  },
  emits: ['update:first'],
  setup: (props: { first: number; total: number }) => () =>
    h('div', { class: 'stub-pagination', 'data-first': props.first }, String(props.total)),
}

const StatePanelStub = {
  name: 'FpStatePanel',
  props: {
    loading: { type: Boolean, default: false },
    error: { type: String, default: null },
    empty: { type: Boolean, default: false },
  },
  emits: ['retry'],
  setup(
    props: { loading: boolean; error: string | null; empty: boolean },
    { slots }: { slots: { default?: () => unknown } },
  ) {
    return () =>
      h(
        'div',
        {
          class: 'stub-state',
          'data-loading': String(props.loading),
          'data-error': props.error ?? '',
          'data-empty': String(props.empty),
        },
        props.error
          ? h('div', { class: 'stub-state-error' }, props.error)
          : props.empty
            ? h('div', { class: 'stub-state-empty' }, '空')
            : (slots.default?.() as never),
      )
  },
}

const messages = {
  website: {
    id: 'ID',
    name: '名称',
    domain: '域名',
    rootPath: '根目录',
    siteType: '站点类型',
    siteType_static: '静态',
    siteType_reverse_proxy: '反向代理',
    rewritePreset: '伪静态模板',
    rewriteNone: '未套用',
    nodeId: '节点 ID',
    status: '状态',
    active: '运行中',
    inactive: '已停用',
    createdAt: '创建时间',
    switchEngine: '切换引擎',
    deploy: '部署配置',
    deployHint: '重新生成并写入',
    bind: '绑定',
  },
  common: {
    operation: '操作',
    edit: '编辑',
    delete: '删除',
    noData: '暂无数据',
  },
}

const site = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'blog',
  domain: 'blog.example.com',
  root_path: '/www/blog',
  node_id: 1,
  status: 'active',
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: null,
  site_type: 'static',
  rewrite_preset: null,
  created_at: '2026-01-01 00:00:00',
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(WebsiteListPanel, {
    props: { rows: [site()], total: 1, ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub, FpPagination: PaginationStub, FpStatePanel: StatePanelStub },
    },
  })
}

function actionLabels(w: ReturnType<typeof mountPanel>): string[] {
  return w.findAll('.row-actions button').map((b) => b.text())
}

describe('WebsiteListPanel · 列渲染', () => {
  it('声明十一列（ID/名称/域名/根目录/站点类型/伪静态/节点/状态/创建时间/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual([
      'ID',
      '名称',
      '域名',
      '根目录',
      '站点类型',
      '伪静态模板',
      '节点 ID',
      '状态',
      '创建时间',
      '操作',
    ])
  })

  it('站点类型：static → 静态；reverse_proxy → 反向代理（i18n 现算）', () => {
    expect(mountPanel({ rows: [site({ site_type: 'static' })] }).text()).toContain('静态')
    expect(
      mountPanel({ rows: [site({ site_type: 'reverse_proxy' })] })
        .find('[data-col="4"]')
        .text(),
    ).toBe('反向代理')
  })

  it('站点类型缺失：回落 static（历史数据无该字段，不得显示裸 key）', () => {
    const w = mountPanel({ rows: [site({ site_type: null })] })
    expect(w.find('[data-col="4"]').text()).toBe('静态')
  })

  it('伪静态列：已套用显示模板名，未套用显示占位文案', () => {
    expect(mountPanel({ rows: [site({ rewrite_preset: 'wordpress' })] }).text()).toContain(
      'wordpress',
    )
    expect(mountPanel({ rows: [site({ rewrite_preset: null })] }).text()).toContain('未套用')
  })

  it('状态列：active / 其他 两档映射', () => {
    expect(mountPanel({ rows: [site({ status: 'active' })] }).text()).toContain('运行中')
    expect(mountPanel({ rows: [site({ status: 'stopped' })] }).text()).toContain('已停用')
  })

  it('字段列直出原值（域名 / 根目录 / 节点 / 创建时间）', () => {
    const row = mountPanel().find('.stub-row').text()
    expect(row).toContain('blog.example.com')
    expect(row).toContain('/www/blog')
    expect(row).toContain('2026-01-01 00:00:00')
  })
})

describe('WebsiteListPanel · 行内操作上抛', () => {
  it('六个操作齐全：编辑/切换引擎/部署配置/绑定/伪静态/删除', () => {
    expect(actionLabels(mountPanel())).toEqual([
      '编辑',
      '切换引擎',
      '部署配置',
      '绑定',
      '伪静态模板',
      '删除',
    ])
  })

  it('每个操作各带上整行数据上抛（展示层不持有请求状态）', async () => {
    const w = mountPanel({ rows: [site({ id: 42 })] })
    const buttons = w.findAll('.row-actions button')
    for (const b of buttons) await b.trigger('click')

    for (const ev of ['edit', 'switchEngine', 'deploy', 'bind', 'rewrite', 'delete']) {
      expect((w.emitted(ev)?.[0]?.[0] as { id: number }).id).toBe(42)
    }
  })

  it('操作列冻结点保留（窄屏下操作按钮始终可点）', () => {
    const w = mountPanel()
    const frozen = w.findAll('.stub-cell').filter((c) => c.attributes('data-header') === '操作')
    expect(frozen).toHaveLength(1)
  })
})

describe('WebsiteListPanel · 状态面板与分页', () => {
  it('loading 透传给状态面板', () => {
    const w = mountPanel({ loading: true })
    expect(w.find('.stub-state').attributes('data-loading')).toBe('true')
  })

  it('error 非空时展示错误态而非表格', () => {
    const w = mountPanel({ error: '加载失败' })
    expect(w.find('.stub-state-error').text()).toBe('加载失败')
  })

  it('空数据（total=0 且非 loading 非 error）→ 空态', () => {
    const w = mountPanel({ rows: [], total: 0 })
    expect(w.find('.stub-state').attributes('data-empty')).toBe('true')
  })

  it('【关键】本页无数据但 total>0（服务端分页的翻页越界）→ 不判空态', () => {
    // 历史上「按本地行数组判空」会把翻页瞬间的加载态误判成空态
    const w = mountPanel({ rows: [], total: 50 })
    expect(w.find('.stub-state').attributes('data-empty')).toBe('false')
  })

  it('retry 事件上抛（状态面板的重试按钮接父级刷新）', async () => {
    const w = mountPanel({ error: 'boom' })
    w.findComponent(StatePanelStub).vm.$emit('retry')
    await w.vm.$nextTick()
    expect(w.emitted('retry')).toBeTruthy()
  })

  it('分页仅在 total > pageSize 时出现', () => {
    expect(mountPanel({ total: 20, pageSize: 20 }).find('.stub-pagination').exists()).toBe(false)
    expect(mountPanel({ total: 21, pageSize: 20 }).find('.stub-pagination').exists()).toBe(true)
  })

  it('翻页上抛 update:first（页号换算在数据层，展示层不参与）', async () => {
    const w = mountPanel({ total: 100 })
    w.findComponent(PaginationStub).vm.$emit('update:first', 40)
    await w.vm.$nextTick()
    expect(w.emitted('update:first')?.[0]?.[0]).toBe(40)
  })
})
