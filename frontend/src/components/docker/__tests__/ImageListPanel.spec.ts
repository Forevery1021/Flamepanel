import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import ImageListPanel from '@/components/docker/ImageListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的镜像列表契约（原为 DockerView 内联表格）。
 *
 * 表格替身同 ContainerListPanel.spec（虚拟滚动在 jsdom 下不渲染，替身按
 * DataTable 公开契约逐行求值）。重点看住**标签列的两级回落**：
 * `tags` → `repo_tags` → 镜像 ID——这是后端 Docker API 形态差异在前端的唯一
 * 缓冲层，回落链断了就是一列空白（且不报错）。
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
    containerId: '容器 ID',
    tags: '标签',
    imageSize: '大小',
    actions: '操作',
    pull: '拉取',
    remove: '删除',
    prune: '清理',
    imageNamePlaceholder: '例如 nginx:latest',
  },
  common: { noData: '暂无数据' },
}

const image = (over: Record<string, unknown> = {}) => ({
  id: 'sha256:abc123456789',
  tags: ['nginx:latest'],
  size: 10485760,
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(ImageListPanel, {
    props: { rows: [image()], ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

describe('ImageListPanel · 列与标签回落', () => {
  it('声明四列（ID/标签/大小/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['容器 ID', '标签', '大小', '操作'])
  })

  it('tags 非空：逐个渲染 FpTag', () => {
    const w = mountPanel({ rows: [image({ tags: ['nginx:latest', 'nginx:1.27'] })] })
    const tags = w.findAll('[data-col="1"] .fp-tag')
    expect(tags.map((t) => t.text())).toEqual(['nginx:latest', 'nginx:1.27'])
  })

  it('tags 为空：回落到 repo_tags（Docker API 原生形态）', () => {
    const w = mountPanel({ rows: [image({ tags: [], repo_tags: ['redis:7', 'redis:alpine'] })] })
    expect(w.find('[data-col="1"]').text()).toBe('redis:7, redis:alpine')
  })

  it('tags 与 repo_tags 都为空：回落镜像 ID（不留空白单元格）', () => {
    const w = mountPanel({ rows: [image({ tags: [], repo_tags: [] })] })
    expect(w.find('[data-col="1"]').text()).toBe('sha256:abc123456789')
  })

  it('大小列经 formatSize 格式化（原始字节 → 人类可读）', () => {
    const w = mountPanel({ rows: [image({ size: 10485760 })] })
    expect(w.find('[data-col="2"]').text()).toBe('10 MB')
  })

  it('大小缺失 → 0 B（而不是 NaN 或 undefined）', () => {
    const w = mountPanel({ rows: [image({ size: undefined })] })
    expect(w.find('[data-col="2"]').text()).toBe('0 B')
  })
})

describe('ImageListPanel · 工具栏与操作上抛', () => {
  it('工具栏三个控件：输入框（回车可提交）、拉取、清理', async () => {
    const w = mountPanel()
    expect(w.find('.toolbar input').attributes('placeholder')).toBe('例如 nginx:latest')

    const buttons = w.findAll('.toolbar button')
    expect(buttons.map((b) => b.text())).toEqual(['拉取', '清理'])
  })

  it('回车与点击拉取按钮都上抛 pull', async () => {
    const w = mountPanel()
    const before = w.emitted('pull')?.length ?? 0

    await w.find('.toolbar input').trigger('keyup.enter')
    expect(w.emitted('pull')?.length ?? 0).toBeGreaterThan(before)

    const afterEnter = w.emitted('pull')?.length ?? 0
    await w.findAll('.toolbar button')[0].trigger('click')
    expect(w.emitted('pull')?.length).toBe(afterEnter + 1)
  })

  it('pulling=true 时拉取按钮进入 loading 且标记为禁用（防重复提交）', () => {
    const w = mountPanel({ pulling: true })
    const btn = w.findAll('.toolbar button')[0]
    expect(btn.classes().join(' ')).toContain('p-button-loading')
    expect(btn.attributes('data-p-disabled')).toBe('true')
  })

  it('行内删除与工具栏清理分别上抛（携带整行 / 无参）', async () => {
    const w = mountPanel()
    // 操作列的删除按钮直接落在单元格里（本表只有一条动作，无需 row-actions 容器）
    await w.find('[data-col="3"] .fp-btn').trigger('click')
    expect(w.emitted('remove')?.[0][0]).toMatchObject({ id: 'sha256:abc123456789' })

    await w.findAll('.toolbar button')[1].trigger('click')
    expect(w.emitted('prune')).toBeTruthy()
  })
})
