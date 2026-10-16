import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode, defineComponent } from 'vue'
import ContainerListPanel from '@/components/docker/ContainerListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的容器列表契约（原为 DockerView 内联表格）。
 *
 * **表格替身（TableStub）**：与 WebsiteListPanel.spec / InstalledAppsPanel.spec 同一取舍——
 * 虚拟滚动依赖真实布局尺寸，而 jsdom 没有排版引擎，真表一行都不渲染。替身按
 * DataTable 的公开契约（`rows` + 各列 `body` 插槽逐行求值）复刻，单元格逻辑得到
 * 真实覆盖；纯对象写法（非 defineComponent）避免 `vue/one-component-per-file` 告警。
 *
 * 本表是五个页签里操作最密的一张（**九类行操作**），且每条的 `disabled` 条件
 * 都是状态机的一部分——它们是本 spec 的重点：禁用条件写错不会报错，只会让用户
 * 对着一个点了没反应的按钮。
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
  docker: {
    name: '名称',
    containerId: '容器 ID',
    image: '镜像',
    status: '状态',
    actions: '操作',
    start: '启动',
    stop: '停止',
    restart: '重启',
    logs: '日志',
    stats: '统计',
    inspect: '查看详情',
    exportCompose: '导出为 Compose',
    rename: '重命名',
    pause: '暂停',
    unpause: '恢复',
    kill: '强制停止',
    remove: '删除',
    prune: '清理',
    searchPlaceholder: '按名称/ID/镜像搜索',
  },
  common: { noData: '暂无数据' },
}

const container = (over: Record<string, unknown> = {}) => ({
  id: 'a1b2c3',
  image: 'nginx:latest',
  name: 'web',
  status: 'running',
  node_id: 1,
  created_at: '2026-01-01T00:00:00Z',
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

/** FpActionMenu 替身：把菜单项平铺成按钮，便于与行内按钮一起断言（真实弹层行为由 FpActionMenu.spec 锁） */
const ActionMenuStub = defineComponent({
  name: 'FpActionMenu',
  props: { items: { type: Array, default: () => [] } },
  template:
    '<button v-for="it in items" :key="it.key" :disabled="it.disabled || undefined" :class="[\'menu-item\', it.loading && \'p-button-loading\']" @click="it.command()">{{ it.label }}</button>',
})

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(ContainerListPanel, {
    props: { rows: [container()], ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { FpTable: TableStub, FpActionMenu: ActionMenuStub },
    },
  })
}

function rowButtons(w: ReturnType<typeof mountPanel>): { text: string; disabled: boolean }[] {
  return w.findAll('.row-actions button').map((b) => ({
    text: b.text(),
    disabled: b.attributes('disabled') !== undefined,
  }))
}

describe('ContainerListPanel · 列渲染', () => {
  it('声明五列（名称/容器 ID/镜像/状态/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['名称', '容器 ID', '镜像', '状态', '操作'])
  })

  it('容器 ID 与镜像列带 tooltip 与截断样式（长值不撑破表格）', () => {
    const w = mountPanel()
    expect(w.find('[data-col="1"] .cell-truncate').text()).toBe('a1b2c3')
    expect(w.find('[data-col="2"] .cell-truncate').text()).toBe('nginx:latest')
  })

  it('状态列：running → success；exited → danger，且仅 running 带呼吸点', () => {
    // 断言落在 FpTag 的**语义 severity 映射产物**上（success/warn/danger 三个等级类），
    // 而不是容器自身的类名——后者只证明组件挂载了，证明不了配色语义。
    const running = mountPanel({ rows: [container({ status: 'running' })] })
    expect(running.find('[data-col="3"] .fp-tag').classes().join(' ')).toContain('success')
    expect(running.find('[data-col="3"] .fp-tag__dot').exists()).toBe(true)

    const exited = mountPanel({ rows: [container({ status: 'exited' })] })
    expect(exited.find('[data-col="3"] .fp-tag').classes().join(' ')).toContain('danger')
    expect(exited.find('[data-col="3"] .fp-tag__dot').exists()).toBe(false)
  })

  it('状态列：paused → warn 等级（底层库命名为 warn 而非 warning）', () => {
    const w = mountPanel({ rows: [container({ status: 'paused' })] })
    expect(w.find('[data-col="3"] .fp-tag').classes().join(' ')).toContain('warn')
  })

  it('空数据渲染空态文案（表格替身的 empty 分支）', () => {
    const w = mountPanel({ rows: [] })
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })
})

describe('ContainerListPanel · 行操作（3 个主按钮 + 「更多」菜单 7 项）', () => {
  it('running 容器：主按钮为「停止」（无「启动」），重启/暂停/强停可用', () => {
    const w = mountPanel({ rows: [container({ status: 'running' })] })
    const btns = rowButtons(w)
    const texts = btns.map((b) => b.text)
    expect(texts.slice(0, 2)).toEqual(['停止', '日志'])
    expect(texts).not.toContain('启动')
    expect(btns.find((b) => b.text === '重启')).toEqual({ text: '重启', disabled: false })
    expect(btns.find((b) => b.text === '暂停')).toEqual({ text: '暂停', disabled: false })
    expect(btns.find((b) => b.text === '强制停止')).toEqual({ text: '强制停止', disabled: false })
  })

  it('exited 容器：主按钮为「启动」（无「停止」），重启/暂停/强停禁用（三个禁用条件各自生效）', () => {
    const w = mountPanel({ rows: [container({ status: 'exited' })] })
    const btns = rowButtons(w)
    const texts = btns.map((b) => b.text)
    expect(texts.slice(0, 2)).toEqual(['启动', '日志'])
    expect(texts).not.toContain('停止')
    expect(btns.find((b) => b.text === '重启')).toEqual({ text: '重启', disabled: true })
    expect(btns.find((b) => b.text === '暂停')).toEqual({ text: '暂停', disabled: true })
    expect(btns.find((b) => b.text === '强制停止')).toEqual({ text: '强制停止', disabled: true })
  })

  it('paused 容器：菜单项变「恢复」且仍可用（暂停态也能操作）', () => {
    const w = mountPanel({ rows: [container({ status: 'paused' })] })
    const btns = rowButtons(w)
    expect(btns.find((b) => b.text === '恢复')).toEqual({ text: '恢复', disabled: false })
    expect(btns.map((b) => b.text)).not.toContain('暂停')
  })

  it('每行 10 个出口（启|停互斥 + 日志 + 8 菜单项）；启停两态合计覆盖全部 11 个事件', () => {
    expect(mountPanel().findAll('.row-actions button')).toHaveLength(10)
    const labels = (st: string) =>
      mountPanel({ rows: [container({ status: st })] })
        .findAll('.row-actions button')
        .map((b) => b.text())
    expect(new Set([...labels('running'), ...labels('exited')]).size).toBe(11)
  })

  it('启停与生命周期操作：在各自可用状态下上抛（携带整行）', async () => {
    const exited = mountPanel({ rows: [container({ status: 'exited' })] })
    await exited.findAll('.row-actions button')[0].trigger('click')
    expect(exited.emitted('start')?.[0][0]).toMatchObject({ id: 'a1b2c3' })

    const running = mountPanel({ rows: [container({ status: 'running' })] })
    const byText = (t: string) =>
      running.findAll('.row-actions button').find((b) => b.text() === t)!
    await byText('停止').trigger('click')
    await byText('暂停').trigger('click')
    await byText('强制停止').trigger('click')
    expect(running.emitted('stop')?.[0][0]).toMatchObject({ id: 'a1b2c3' })
    expect(running.emitted('pause')?.[0][0]).toMatchObject({ id: 'a1b2c3' })
    expect(running.emitted('kill')?.[0][0]).toMatchObject({ id: 'a1b2c3' })
  })

  it('无状态门槛的操作：日志 / 统计 / 详情 / 导出 Compose / 重命名 / 删除均可直接上抛', async () => {
    const w = mountPanel({ rows: [container({ status: 'running' })] })
    const byText = (t: string) => w.findAll('.row-actions button').find((b) => b.text() === t)!
    for (const label of ['重启', '日志', '统计', '查看详情', '导出为 Compose', '重命名', '删除'])
      await byText(label).trigger('click')
    for (const name of [
      'restart',
      'logs',
      'stats',
      'detail',
      'export-compose',
      'rename',
      'remove',
    ]) {
      expect(w.emitted(name)?.[0][0], `未上抛 ${name}`).toMatchObject({ id: 'a1b2c3' })
    }
  })

  it('清理按钮上抛 prune（工具栏，非行内）', async () => {
    const w = mountPanel()
    await w.find('.toolbar button').trigger('click')
    expect(w.emitted('prune')).toBeTruthy()
  })
})
