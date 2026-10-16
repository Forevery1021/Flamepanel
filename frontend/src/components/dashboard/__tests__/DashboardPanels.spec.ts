import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h } from 'vue'
import StatCardGrid from '@/components/dashboard/StatCardGrid.vue'
import ProcessPanel from '@/components/dashboard/ProcessPanel.vue'
import TodoPanel from '@/components/dashboard/TodoPanel.vue'
import CommonAppsPanel from '@/components/dashboard/CommonAppsPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import type { DashboardStatCard } from '@/utils/dashboardFormat'

/**
 * T2-4 第五站的展示层契约（原为 DashboardView 内联模板）。
 *
 * 四块面板的重点各不相同，但共同点都是「界面行为」——这些在拆分前混在 851 行里，
 * 只有靠读模板才能重建：
 * - 指标卡：可点击性由 `to` 决定、按钮语义 + aria-label、进度条值与类目色；
 * - 进程表：四列的格式化（CPU 一位小数、内存 MB、PID 等宽）；
 * - 待办：viewer 只读时**不渲染**勾选框、骨架屏与空态互斥、勾选上抛布尔；
 * - 常用应用：次数缺失回落 0、viewer 时按钮置灰、空态文案。
 */

const messages = {
  dashboard: {
    cpu: 'CPU',
    memory: '内存',
    disk: '磁盘',
    load: '系统负载',
    goTo: '查看{module}详情',
    waitingData: '等待数据…',
    processTop: '进程 TOP',
    processName: '进程',
    processCpu: 'CPU',
    processMem: '内存',
    processPid: 'PID',
    todayTodos: '今日待办',
    more: '更多',
    noTodos: '今日暂无待办',
    commonApps: '常用应用',
    noCommonApps: '暂无已安装应用',
  },
  common: { noData: '暂无数据' },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(
  component: unknown,
  props: Record<string, unknown> = {},
  global: Record<string, unknown> = {},
) {
  return mountLoose(component, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      ...global,
    },
  })
}

/** 表格替身：虚拟滚动在 jsdom 下一行不渲染（无排版引擎），按 DataTable 公开契约逐行求值 */
type ColumnVNode = {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = {
  name: 'FpTable',
  props: { rows: { type: Array, default: () => [] } },
  setup(
    props: { rows: Record<string, unknown>[] },
    { slots }: { slots: { default?: () => unknown } },
  ) {
    return () => {
      const columns = (slots.default?.() ?? []) as ColumnVNode[]
      return h(
        'div',
        { class: 'stub-table' },
        props.rows.map((row, index) =>
          h(
            'div',
            { class: 'stub-row', 'data-p-index': index },
            columns.map((col, colIndex) =>
              h(
                'div',
                {
                  class: 'stub-cell',
                  'data-col': colIndex,
                  'data-header': String(col.props?.header ?? ''),
                },
                col.children?.body
                  ? (col.children.body({ data: row }) as never)
                  : String(col.props?.field ? (row[col.props.field] ?? '') : ''),
              ),
            ),
          ),
        ),
      )
    }
  },
}

const card = (over: Partial<DashboardStatCard> = {}): DashboardStatCard => ({
  key: 'cpu',
  icon: 'oi-microchip',
  label: 'CPU',
  value: '12.3%',
  percent: 12,
  detail: '8 cores',
  color: 'var(--fp-success)',
  to: '/nodes',
  ...over,
})

const todo = (over: Record<string, unknown> = {}) => ({
  id: 1,
  content: '写周报',
  kind: 'todo',
  done: false,
  created_at: '',
  updated_at: '',
  ...over,
})

describe('StatCardGrid · 指标卡网格', () => {
  it('按 cards 渲染四张卡，类名与 key 对应（配色类）', () => {
    const w = mountPanel(StatCardGrid, {
      cards: [
        card({ key: 'cpu' }),
        card({ key: 'mem' }),
        card({ key: 'disk' }),
        card({ key: 'load' }),
      ],
    })
    expect(w.findAll('.stat-card')).toHaveLength(4)
    expect(w.findAll('.stat-card').map((c) => c.classes().join(' '))).toEqual([
      expect.stringContaining('stat-cpu'),
      expect.stringContaining('stat-mem'),
      expect.stringContaining('stat-disk'),
      expect.stringContaining('stat-load'),
    ])
  })

  it('开屏：pending 时数值位显示骨架、明细显示等待文案（不显示误导性的 0%）', () => {
    const w = mountPanel(StatCardGrid, {
      cards: [card({ key: 'cpu', value: '0.0%', detail: '0 cores' })],
      pending: true,
    })
    expect(w.find('.stat-skeleton').exists()).toBe(true)
    expect(w.find('.stat-value').text()).toBe('')
    expect(w.find('.stat-detail').text()).toContain('等待数据')
    expect(w.find('.stats-grid').attributes('aria-busy')).toBe('true')
  })

  it('迷你趋势线：≥2 点历史才画，且只对四个已知 key 生成 path', () => {
    const hist = (n: number) =>
      Array.from({ length: n }, (_, i) => ({ cpu_usage: i * 10, memory_usage_percent: 50 }))
    const one = mountPanel(StatCardGrid, { cards: [card({ key: 'cpu' })], history: hist(1) })
    expect(one.find('.stat-spark').exists()).toBe(false)
    const many = mountPanel(StatCardGrid, {
      cards: [card({ key: 'cpu' }), card({ key: 'mem' })],
      history: hist(5),
    })
    const paths = many.findAll('.stat-spark path')
    expect(paths).toHaveLength(2)
    // CPU 单调上升：首点在底部（y=26）、末点在顶部（y=2）
    expect(paths[0].attributes('d')).toMatch(/^M0\.0,26\.0 .*L100\.0,2\.0$/)
    // 内存恒定：span 归一为 1 → 全部落在底线
    expect(paths[1].attributes('d')).toMatch(/^M0\.0,26\.0( L[\d.]+,26\.0)+$/)
  })

  it('渲染数值与明细（用户可见的两行）', () => {
    const w = mountPanel(StatCardGrid, { cards: [card({ value: '77.7%', detail: '9 cores' })] })
    expect(w.find('.stat-value').text()).toBe('77.7%')
    expect(w.find('.stat-detail').text()).toBe('9 cores')
  })

  it('可跳转的卡片：aria-label 走 goTo 插值 + 点击上抛目标', async () => {
    const w = mountPanel(StatCardGrid, { cards: [card({ to: '/nodes' })] })
    const btn = w.find('.stat-card')
    expect(btn.attributes('aria-label')).toBe('查看CPU详情')
    expect(btn.classes()).toContain('is-clickable')
    await btn.trigger('click')
    expect(w.emitted('navigate')).toEqual([['/nodes']])
  })

  it('无跳转目标：不可点击、不渲染箭头、点击不上抛', async () => {
    const w = mountPanel(StatCardGrid, { cards: [card({ to: '' })] })
    const btn = w.find('.stat-card')
    expect(btn.attributes('aria-label')).toBe('CPU')
    expect(btn.classes()).not.toContain('is-clickable')
    expect(w.find('.stat-go').exists()).toBe(false)
    await btn.trigger('click')
    expect(w.emitted('navigate')).toBeUndefined()
  })

  it('进度条收到 percent（数组顺序即卡序）', () => {
    const w = mountPanel(StatCardGrid, {
      cards: [card({ key: 'cpu', percent: 12 }), card({ key: 'mem', percent: 64 })],
    })
    // 自绘色阶进度条：宽度 = percent，色阶按阈值分档（<60 ok / <85 warn / ≥85 bad）
    const fills = w.findAll('.stat-bar__fill')
    expect(fills).toHaveLength(2)
    expect(fills[0].attributes('style')).toContain('width: 12%')
    expect(fills[0].classes()).toContain('tone-ok')
    expect(fills[1].attributes('style')).toContain('width: 64%')
    expect(fills[1].classes()).toContain('tone-warn')
  })
})

describe('ProcessPanel · 进程表', () => {
  it('四列：进程 / CPU / 内存 / PID（列头与顺序）', () => {
    const w = mountPanel(
      ProcessPanel,
      { rows: [{ name: 'nginx', cpu: 1, memory_mb: 2, pid: 3, status: 'running' }] },
      { stubs: { FpTable: TableStub } },
    )
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['进程', 'CPU', '内存', 'PID'])
  })

  it('CPU 保留一位小数并带 %（浮点噪声不出屏）', () => {
    const w = mountPanel(
      ProcessPanel,
      { rows: [{ name: 'nginx', cpu: 0.30000000000000004, memory_mb: 12, pid: 42, status: 'running' }] },
      { stubs: { FpTable: TableStub } },
    )
    expect(w.find('.stub-table').exists()).toBe(true)
    const cells = w.findAll('.stub-row .stub-cell')
    expect(cells[1].text()).toBe('0.3%')
  })

  it('内存带 MB 后缀、PID 原样（不换算、不补零）', () => {
    const w = mountPanel(
      ProcessPanel,
      { rows: [{ name: 'nginx', cpu: 1, memory_mb: 2048, pid: 7, status: 'running' }] },
      { stubs: { FpTable: TableStub } },
    )
    const cells = w.findAll('.stub-row .stub-cell')
    expect(cells[2].text()).toBe('2048 MB')
    expect(cells[3].text()).toBe('7')
  })
})

describe('TodoPanel · 今日待办', () => {
  it('loading 出骨架、不出空态（避免先闪「暂无待办」）', () => {
    const w = mountPanel(TodoPanel, { todos: [], loading: true, editable: true })
    expect(w.findAll('.todo-skeleton > *').length).toBeGreaterThan(0)
    expect(w.find('.panel-empty').exists()).toBe(false)
  })

  it('非 loading 且空列表：出空态文案', () => {
    const w = mountPanel(TodoPanel, { todos: [], loading: false, editable: true })
    expect(w.find('.panel-empty').text()).toBe('今日暂无待办')
    expect(w.find('.todo-skeleton').exists()).toBe(false)
  })

  it('editable：渲染勾选框，change 上抛布尔（含取消勾选）', async () => {
    const w = mountPanel(TodoPanel, { todos: [todo()], loading: false, editable: true })
    const box = w.find('input[type="checkbox"]')
    expect(box.exists()).toBe(true)
    await box.setValue(true)
    expect(w.emitted('toggle')?.[0]?.[1]).toBe(true)
  })

  it('viewer 只读：不渲染勾选框（不是灰掉）', () => {
    const w = mountPanel(TodoPanel, { todos: [todo()], loading: false, editable: false })
    expect(w.find('input[type="checkbox"]').exists()).toBe(false)
    expect(w.find('.todo-content').text()).toBe('写周报')
  })

  it('已完成项带 done 类（删除线）', () => {
    const w = mountPanel(TodoPanel, { todos: [todo({ done: true })], loading: false, editable: true })
    expect(w.find('.todo-item').classes()).toContain('done')
  })

  it('「更多」上抛 more（装配层跳备忘录页）', async () => {
    const w = mountPanel(TodoPanel, { todos: [todo()], loading: false, editable: true })
    await w.find('button').trigger('click')
    expect(w.emitted('more')).toHaveLength(1)
  })
})

describe('CommonAppsPanel · 常用应用', () => {
  it('渲染应用名与启动次数', () => {
    const w = mountPanel(CommonAppsPanel, {
      apps: [{ id: 1, name: 'Nginx', launch_count: 5 }],
      disabled: false,
    })
    expect(w.find('.common-app-name').text()).toBe('Nginx')
    expect(w.find('.common-app-count').text()).toBe('5')
  })

  it('缺 launch_count 回落 0（不渲染空白列）', () => {
    const w = mountPanel(CommonAppsPanel, { apps: [{ id: 1, name: 'Nginx' }], disabled: false })
    expect(w.find('.common-app-count').text()).toBe('0')
  })

  it('空列表出空态文案', () => {
    const w = mountPanel(CommonAppsPanel, { apps: [], disabled: false })
    expect(w.find('.panel-empty').text()).toBe('暂无已安装应用')
  })

  it('viewer 只读：按钮置灰（清单仍可见，只是不能点）', async () => {
    const w = mountPanel(CommonAppsPanel, {
      apps: [{ id: 1, name: 'Nginx', launch_count: 1 }],
      disabled: true,
    })
    const btn = w.find('.common-app')
    expect(btn.attributes('disabled')).toBeDefined()
    await btn.trigger('click')
    expect(w.emitted('open')).toBeUndefined()
  })

  it('可点击时上抛整行应用对象', async () => {
    const app = { id: 1, name: 'Nginx', launch_count: 1 }
    const w = mountPanel(CommonAppsPanel, { apps: [app], disabled: false })
    await w.find('.common-app').trigger('click')
    expect(w.emitted('open')?.[0]?.[0]).toEqual(app)
  })
})
