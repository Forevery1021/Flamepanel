import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, type PropType, type VNode } from 'vue'
import InstalledAppsPanel from '@/components/appStore/InstalledAppsPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分后的已安装应用表契约（原为 AppStoreView 内联表格）。
 *
 * **为什么用表格替身（TableStub）而不是真 FpTable**：本表启用虚拟滚动，而虚拟滚动
 * 依赖真实布局尺寸——jsdom 没有排版引擎（T1-1 已记录该约束），真表在 jsdom 下
 * **一行都不渲染**，断言只能得到空 tbody。替身按 DataTable 的公开契约渲染：
 * 遍历列声明的 `body` 作用域插槽逐行求值（这正是真实表格做的事），
 * 因此单元格逻辑（模式/状态标签、访问链接、原生按钮禁用态）得到真实覆盖，
 * 同时不伪造任何布局假设。
 *
 * 关注点：列语义、缺失值占位、原生模式专属按钮的**渲染条件**与禁用态、
 * 六个操作事件的上抛（展示层不持有请求状态）。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

/** 列 VNode 的窄化视图：只需读取 props 与 body 插槽（替身表格的求值口径） */
type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

/**
 * 表格替身：按列声明的 `body` 插槽逐行求值。这是 DataTable 两段公开契约的
 * 忠实复刻——① `rows` 为数据源；② 每一列的单元格内容来自该列的 `body` 插槽。
 * 未声明 `body` 的列（字段列）直接读字段值（与真实表格一致）。
 */
const TableStub = defineComponent({
  name: 'FpTable',
  props: {
    rows: { type: Array as PropType<Record<string, unknown>[]>, default: () => [] },
    loading: { type: Boolean, default: false },
    emptyText: { type: String, default: '' },
  },
  setup(props, { slots }) {
    return () => {
      const columns = (slots.default?.() ?? []) as ColumnVNode[]
      if (!props.rows.length) {
        return h('div', { class: 'stub-empty' }, props.emptyText)
      }
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
                body
                  ? (body({ data: row }) as never)
                  : String(field ? (row[field] ?? '') : ''),
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
    empty: '暂无应用',
    colName: '名称',
    colPackage: '应用包',
    colVersion: '版本',
    colMode: '模式',
    colStatus: '状态',
    colAccess: '访问地址',
    colActions: '操作',
    start: '启动',
    stop: '停止',
    restart: '重启',
    logs: '日志',
    upgrade: '升级',
    uninstall: '卸载',
    modeContainer: '容器',
    modeNative: '原生',
  },
}

const app = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'Gitea',
  package_key: 'gitea',
  version: '1.0.0',
  mode: 'container',
  status: 'running',
  access_url: 'http://localhost:3000',
  install_path: '/opt/apps/gitea',
  params_json: '{}',
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

function mountPanel(apps: Record<string, unknown>[]) {
  return mountLoose(InstalledAppsPanel, {
    props: { apps },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

/** 操作列（`.row-actions`）内的按钮文案 */
function actionLabels(w: ReturnType<typeof mountPanel>): string[] {
  return w.findAll('.row-actions button').map((b) => b.text())
}

describe('InstalledAppsPanel · 单元格渲染', () => {
  it('声明七列：名称/应用包/版本/模式/状态/访问地址/操作', () => {
    const w = mountPanel([app()])
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual([
      '名称',
      '应用包',
      '版本',
      '模式',
      '状态',
      '访问地址',
      '操作',
    ])
  })

  it('容器模式：模式列走 i18n、状态列直出原值、访问地址为可点链接', () => {
    const w = mountPanel([app()])
    expect(w.find('.stub-row').text()).toContain('容器')
    expect(w.find('.stub-row').text()).toContain('running')
    const link = w.find('.access-link')
    expect(link.attributes('href')).toBe('http://localhost:3000')
    expect(link.attributes('target')).toBe('_blank')
  })

  it('访问地址缺失 → `-` 占位（不出空链接）', () => {
    const w = mountPanel([app({ access_url: null })])
    expect(w.find('.access-link').exists()).toBe(false)
    expect(w.find('[data-col="5"]').text()).toBe('-')
  })

  it('原生模式：模式列显示「原生」，且出现启停按钮', () => {
    const w = mountPanel([app({ mode: 'native', status: 'running' })])
    expect(w.find('.stub-row').text()).toContain('原生')
    expect(actionLabels(w)).toEqual(['启动', '停止', '重启', '日志', '升级', '卸载'])
  })

  it('容器模式：无原生生命周期按钮（只保留日志/升级/卸载）', () => {
    const w = mountPanel([app()])
    expect(actionLabels(w)).toEqual(['日志', '升级', '卸载'])
  })

  it('空列表 → 空态文案由表格的 empty 槽承载', () => {
    const w = mountPanel([])
    expect(w.find('.stub-empty').text()).toBe('暂无应用')
  })
})

describe('InstalledAppsPanel · 原生按钮禁用态', () => {
  it('running：启动禁用、停止可用（不得对已运行实例重复启动）', () => {
    const w = mountPanel([app({ mode: 'native', status: 'running' })])
    const buttons = w.findAll('.row-actions button')
    expect(buttons[0].attributes('disabled')).toBeDefined()
    expect(buttons[1].attributes('disabled')).toBeUndefined()
  })

  it('stopped：启动可用、停止禁用', () => {
    const w = mountPanel([app({ mode: 'native', status: 'stopped' })])
    const buttons = w.findAll('.row-actions button')
    expect(buttons[0].attributes('disabled')).toBeUndefined()
    expect(buttons[1].attributes('disabled')).toBeDefined()
  })
})

describe('InstalledAppsPanel · 事件上抛（展示层不持有请求状态）', () => {
  it('容器模式三个操作各带上整行数据上抛', async () => {
    const w = mountPanel([app()])
    const buttons = w.findAll('.row-actions button')
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')

    expect((w.emitted('logs')?.[0]?.[0] as { id: number }).id).toBe(1)
    expect((w.emitted('upgrade')?.[0]?.[0] as { id: number }).id).toBe(1)
    expect((w.emitted('uninstall')?.[0]?.[0] as { id: number }).id).toBe(1)
  })

  it('原生模式额外上抛启停与重启（按当前状态点可用的那个）', async () => {
    const w = mountPanel([app({ mode: 'native', status: 'stopped' })])
    const buttons = w.findAll('.row-actions button')
    await buttons[0].trigger('click') // 启动（stopped 时可用）
    await buttons[2].trigger('click') // 重启
    await buttons[3].trigger('click') // 日志
    expect(w.emitted('nativeStart')).toBeTruthy()
    expect(w.emitted('nativeRestart')).toBeTruthy()
    expect(w.emitted('logs')).toBeTruthy()
  })

  it('禁用按钮不产生事件（禁用态是真实可交互性，不只是视觉置灰）', async () => {
    const w = mountPanel([app({ mode: 'native', status: 'stopped' })])
    await w.findAll('.row-actions button')[1].trigger('click') // 停止：stopped 时禁用
    expect(w.emitted('nativeStop')).toBeUndefined()
  })

  it('running 状态点「停止」上抛 nativeStop（可停才可停）', async () => {
    const w = mountPanel([app({ mode: 'native', status: 'running' })])
    await w.findAll('.row-actions button')[1].trigger('click')
    expect(w.emitted('nativeStop')).toBeTruthy()
  })
})
