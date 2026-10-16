import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import VolumeListPanel from '@/components/docker/VolumeListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的卷列表契约（原为 DockerView 内联表格）。
 *
 * 三列 + 一个删除动作，看似「没什么可测」——但挂载点列是卷页签存在的理由
 * （回答「这个卷挂到哪去了」），列语义与顺序即契约。
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
    mountpoint: '挂载点',
    actions: '操作',
    remove: '删除',
    prune: '清理',
    createVolume: '创建卷',
  },
  common: { noData: '暂无数据' },
}

const volume = (over: Record<string, unknown> = {}) => ({
  name: 'app-data',
  driver: 'local',
  mountpoint: '/var/lib/docker/volumes/app-data/_data',
  created_at: null,
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(VolumeListPanel, {
    props: { rows: [volume()], ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

describe('VolumeListPanel', () => {
  it('声明四列（名称/驱动/挂载点/操作）与字段值直出', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['名称', '驱动', '挂载点', '操作'])
    expect(w.find('[data-col="2"]').text()).toBe('/var/lib/docker/volumes/app-data/_data')
  })

  it('工具栏两个入口：创建卷 + 清理', () => {
    const w = mountPanel()
    expect(w.findAll('.toolbar button').map((b) => b.text())).toEqual(['创建卷', '清理'])
  })

  it('删除上抛整行（数据层按 name 寻址——本行的值即契约）', async () => {
    const w = mountPanel()
    // 本表只有一个动作，按钮直接落在操作列单元格里（无 row-actions 容器）
    await w.find('[data-col="3"] .fp-btn').trigger('click')
    expect(w.emitted('remove')?.[0][0]).toMatchObject({ name: 'app-data' })
  })

  it('创建与清理各自上抛', async () => {
    const w = mountPanel()
    await w.findAll('.toolbar button')[0].trigger('click')
    await w.findAll('.toolbar button')[1].trigger('click')
    expect(w.emitted('create')).toBeTruthy()
    expect(w.emitted('prune')).toBeTruthy()
  })

  it('空数据渲染空态（替身表格的 empty 分支）', () => {
    const w = mountPanel({ rows: [] })
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })
})
