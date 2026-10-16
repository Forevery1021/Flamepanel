import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, type PropType, type VNode } from 'vue'
import SourcesPanel from '@/components/appStore/SourcesPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分后的远程源列表契约（原为 AppStoreView 内联表格）。
 *
 * 表格替身理由同 InstalledAppsPanel.spec（虚拟滚动 jdom 不可渲染，见该文件注释）：
 * 按 DataTable 的公开契约（rows + 列 body 插槽）逐行求值，单元格逻辑真实覆盖。
 *
 * 关注点：验签公钥状态列（已配置/未配置）、同步按钮 loading 归属到正确行、
 * 四个操作事件上抛、时间列缺省占位。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

/** 列 VNode 的窄化视图：只需读取 props 与 body 插槽（替身表格的求值口径） */
type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = defineComponent({
  name: 'FpTable',
  props: {
    rows: { type: Array as PropType<Record<string, unknown>[]>, default: () => [] },
    emptyText: { type: String, default: '' },
  },
  setup(props, { slots }) {
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
                { class: 'stub-cell', 'data-col': colIndex, 'data-header': String(col.props?.header ?? '') },
                body ? (body({ data: row }) as never) : String(field ? (row[field] ?? '') : ''),
              )
            }),
          ),
        ),
      )
    }
  },
})

const messages = {
  appStore: {
    addSource: '添加远程源',
    emptySources: '暂无远程源',
    colSourceName: '名称',
    colSourceType: '类型',
    colSourceUrl: '地址',
    colSourceKey: '验签公钥',
    colSyncedAt: '最近同步',
    colActions: '操作',
    keyConfigured: '已配置',
    keyMissing: '未配置',
    syncNow: '同步',
    editPublicKey: '公钥',
    enableSource: '启用',
    disableSource: '停用',
    deleteSource: '删除',
  },
}

const source = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'official',
  source_type: 'git',
  url: 'https://example.com/appstore',
  public_key: 'KEY',
  enabled: true,
  last_synced_at: '2026-09-14T02:00:00Z',
  created_at: '',
  updated_at: '',
  ...over,
})

/**
 * 宽松挂载：`mount` 的泛型会按被测组件的精确 props 约束 options，而替身表格与
 * 用例夹具刻意使用超集/子集数据（与 `mountFp` 同一取舍，仅此一处 unsound）。
 */
const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown>) {
  return mountLoose(SourcesPanel, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

describe('SourcesPanel · 列表渲染', () => {
  it('六列：名称/类型/地址/验签公钥/最近同步/操作', () => {
    const w = mountPanel({ sources: [source()] })
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['名称', '类型', '地址', '验签公钥', '最近同步', '操作'])
  })

  it('类型列大写展示（git → GIT）', () => {
    const w = mountPanel({ sources: [source({ source_type: 'http' })] })
    expect(w.find('[data-col="1"]').text()).toBe('HTTP')
  })

  it('验签公钥列：有公钥「已配置」成功色；无公钥「未配置」警示色', () => {
    const ok = mountPanel({ sources: [source({ public_key: 'k' })] })
    expect(ok.find('[data-col="3"]').text()).toContain('已配置')

    const missing = mountPanel({ sources: [source({ public_key: null })] })
    expect(missing.find('[data-col="3"]').text()).toContain('未配置')
  })

  it('最近同步时间缺失 → `-` 占位（不渲染 Invalid Date）', () => {
    const w = mountPanel({ sources: [source({ last_synced_at: null })] })
    expect(w.find('[data-col="4"]').text()).toBe('-')
  })

  it('空列表 → 空态文案', () => {
    const w = mountPanel({ sources: [] })
    expect(w.find('.stub-empty').text()).toBe('暂无远程源')
  })

  it('停用的源：操作列显示「启用」（按钮文案由 enabled 决定）', () => {
    const w = mountPanel({ sources: [source({ enabled: false })] })
    expect(w.find('[data-col="5"]').text()).toContain('启用')
    expect(w.find('[data-col="5"]').text()).not.toContain('停用')
  })

  it('同步中：仅对应行的同步按钮进入 loading（其余行不受影响）', () => {
    const w = mountPanel({
      sources: [source({ id: 3 }), source({ id: 4, name: 'other' })],
      syncingSourceId: 3,
    })
    const rows = w.findAll('.stub-row')
    // 第一行第一个按钮（同步）处于 loading；第二行不是
    expect(rows[0].findAll('button')[0].html()).not.toBe(rows[1].findAll('button')[0].html())
  })
})

describe('SourcesPanel · 事件上抛', () => {
  it('顶部按钮上抛 create（弹窗状态由视图层管）', async () => {
    const w = mountPanel({ sources: [] })
    await w.find('.sources-toolbar button').trigger('click')
    expect(w.emitted('create')).toBeTruthy()
  })

  it('行内四操作各带上整行数据上抛', async () => {
    const w = mountPanel({ sources: [source()] })
    const buttons = w.findAll('[data-col="5"] button')
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')
    await buttons[3].trigger('click')

    for (const evt of ['sync', 'editKey', 'toggle', 'remove'] as const) {
      expect((w.emitted(evt)?.[0]?.[0] as { id: number }).id).toBe(3)
    }
  })
})
