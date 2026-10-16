import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import FileTable from '@/components/files/FileTable.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * FilesView 拆出的文件表格。表格替身同 ContainerListPanel.spec（jsdom 无排版，虚拟滚动真表不出行）；
 * 行操作菜单替身把每个 item 渲染成按钮，直接验证菜单内容与 `action` 事件。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = {
  name: 'FpTable',
  props: { rows: { type: Array, default: () => [] } },
  emits: ['row-dblclick'],
  setup(
    props: { rows: Record<string, unknown>[] },
    { slots, emit }: { slots: { default?: () => unknown }; emit: (e: string, p: unknown) => void },
  ) {
    return () => {
      const columns = (slots.default?.() ?? []) as ColumnVNode[]
      return h(
        'div',
        { class: 'stub-table' },
        props.rows.map((row) =>
          h(
            'div',
            { class: 'stub-row', onDblclick: () => emit('row-dblclick', { data: row }) },
            columns.map((col) => {
              const field = col.props?.field
              const body = col.children?.body
              return h(
                'div',
                { class: 'stub-cell' },
                body ? (body({ data: row }) as never) : String(field ? (row[field] ?? '') : ''),
              )
            }),
          ),
        ),
      )
    }
  },
}

const MenuStub = {
  name: 'FpActionMenu',
  props: { items: { type: Array, default: () => [] }, ariaLabel: { type: String, default: '' } },
  template:
    '<div class="stub-menu" :aria-label="ariaLabel"><button v-for="i in items" :key="i.key" class="menu-item" :data-key="i.key" @click="i.command()">{{ i.label }}</button></div>',
}

const messages = {
  common: { noData: '暂无数据' },
  file: {
    selectAll: '全选',
    selectRow: '选择 {name}',
    name: '名称',
    size: '大小',
    permissions: '权限',
    modified: '修改时间',
    actions: '操作',
    edit: '编辑',
    download: '下载',
    moreFor: '{name} 的更多操作',
    symlink: '链接',
    rename: '重命名',
    chmod: '修改权限',
    compress: '压缩',
    copyTo: '复制到',
    moveTo: '移动到',
    extract: '解压',
    delete: '删除',
    permanentDelete: '永久删除',
  },
}

const entries = [
  {
    name: 'etc',
    path: '/etc',
    is_dir: true,
    is_symlink: false,
    size: 0,
    permissions: '755',
    modified_at: '2026-01-01T00:00:00Z',
  },
  {
    name: 'a.zip',
    path: '/a.zip',
    is_dir: false,
    is_symlink: true,
    size: 2048,
    permissions: '644',
    modified_at: 'bad',
  },
]

function mountTable(over: Record<string, unknown> = {}) {
  return mount(FileTable, {
    props: {
      entries,
      loading: false,
      selected: ['/etc'],
      allSelected: false,
      downloading: '',
      ...over,
    } as never,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub as never, FpActionMenu: MenuStub },
    },
  })
}

describe('FileTable', () => {
  it('渲染名称 / 图标 / 大小 / 链接标记与逐行唯一的菜单无障碍名称', () => {
    const w = mountTable()
    const rows = w.findAll('.stub-row')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.find('.file-name i').classes()).toContain('oi-folder')
    expect(rows[0]!.text()).toContain('-')
    expect(rows[0]!.text()).not.toContain('编辑')
    expect(rows[1]!.find('.file-name i').classes()).toContain('oi-box')
    expect(rows[1]!.text()).toContain('链接')
    expect(rows[1]!.text()).toContain('2')
    expect(rows[1]!.text()).toContain('—')
    expect(rows[1]!.find('.stub-menu').attributes('aria-label')).toBe('a.zip 的更多操作')
  })

  it('点击 / 双击名称 emit open；编辑、下载按钮 emit 对应事件', async () => {
    const w = mountTable()
    await w.findAll('.file-name')[0]!.trigger('click')
    await w.findAll('.stub-row')[1]!.trigger('dblclick')
    expect(w.emitted('open')!.map((e) => (e[0] as { path: string }).path)).toEqual([
      '/etc',
      '/a.zip',
    ])
    const buttons = w.findAll('.stub-row')[1]!.findAll('button')
    await buttons.find((b) => b.text() === '编辑')!.trigger('click')
    await buttons.find((b) => b.text() === '下载')!.trigger('click')
    expect(w.emitted('edit')).toHaveLength(1)
    expect(w.emitted('download')).toHaveLength(1)
  })

  it('行菜单：压缩包有解压项，点击 emit action(key,row)', async () => {
    const w = mountTable()
    const dirKeys = w
      .findAll('.stub-row')[0]!
      .findAll('.menu-item')
      .map((b) => b.attributes('data-key'))
    expect(dirKeys).not.toContain('extract')
    const zipMenu = w.findAll('.stub-row')[1]!.findAll('.menu-item')
    expect(zipMenu.map((b) => b.attributes('data-key'))).toContain('extract')
    await zipMenu.find((b) => b.attributes('data-key') === 'purge')!.trigger('click')
    const [key, row] = w.emitted('action')![0] as [string, { path: string }]
    expect(key).toBe('purge')
    expect(row.path).toBe('/a.zip')
  })

  it('勾选框反映选中状态并 emit toggle', async () => {
    const w = mountTable()
    const boxes = w.findAll('.stub-row input[type="checkbox"]')
    expect((boxes[0]!.element as HTMLInputElement).checked).toBe(true)
    await boxes[1]!.setValue(true)
    expect(w.emitted('toggle')![0]).toEqual(['/a.zip', true])
  })
})
