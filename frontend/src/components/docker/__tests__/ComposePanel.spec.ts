import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import ComposePanel from '@/components/docker/ComposePanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的 Compose 面板契约（原为 DockerView 内联表单 + 表格）。
 *
 * 本面板有两处**共用同一个 `name`** 的 up/down 入口（表单上的一组、项目表行内的一组）——
 * 这是原设计（表单里的项目名既是部署目标也是启停目标），拆出后由 `v-model:name`
 * 承载，语义反而更显式。用例分别锁死两条路径互不串扰。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

const TableStub = {
  name: 'FpTable',
  props: { rows: { type: Array, default: () => [] }, emptyText: { type: String, default: '' } },
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
    projectName: '项目名称',
    yaml: 'Compose YAML',
    deploy: '部署',
    up: '启动',
    down: '停止',
    projects: 'Compose 项目',
    projectStatus: '状态',
    projectConfigFiles: '配置文件',
    actions: '操作',
  },
  common: { noData: '暂无数据', placeholder: '请输入' },
}

const project = (over: Record<string, unknown> = {}) => ({
  name: 'myapp',
  status: 'running(1)',
  config_files: '/opt/myapp/compose.yaml',
  ...over,
})

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(ComposePanel, {
    props: { rows: [project()], name: 'myapp', yaml: 'services: {}', ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

describe('ComposePanel · 部署表单', () => {
  it('表单三件套：项目名输入框、YAML 文本域、三个动作按钮', () => {
    const w = mountPanel()
    expect(w.findAll('.form-actions button').map((b) => b.text())).toEqual(['部署', '启动', '停止'])
    expect(w.find('textarea').exists()).toBe(true)
  })

  it('名称为空：表单上的启动/停止按钮禁用（这两个动作必须有目标）', () => {
    const w = mountPanel({ name: '' })
    const btns = w.findAll('.form-actions button')
    expect(btns[1].attributes('data-p-disabled')).toBe('true')
    expect(btns[2].attributes('data-p-disabled')).toBe('true')
    // 部署按钮不禁用：空值由数据层的前置校验拦（给 warning 而不是「点了没反应」）
    expect(w.emitted('deploy')).toBeUndefined()
  })

  it('提交表单上抛 deploy（无参——载荷经 v-model 由父级持有）', async () => {
    const w = mountPanel()
    await w.find('form').trigger('submit')
    expect(w.emitted('deploy')).toBeTruthy()
  })

  it('表单按钮上抛 up/down 且携带当前表单项目名', async () => {
    const w = mountPanel({ name: 'form-app' })
    const btns = w.findAll('.form-actions button')
    await btns[1].trigger('click')
    await btns[2].trigger('click')

    expect(w.emitted('up')?.[0]).toEqual(['form-app'])
    expect(w.emitted('down')?.[0]).toEqual(['form-app'])
  })

  it('submitting=true：部署按钮进入 loading（防重复提交）', () => {
    const w = mountPanel({ submitting: true })
    expect(w.findAll('.form-actions button')[0].classes().join(' ')).toContain('p-button-loading')
  })
})

describe('ComposePanel · 项目表', () => {
  it('声明四列（项目名/状态/配置文件/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['项目名称', '状态', '配置文件', '操作'])
  })

  it('行内 up/down 上抛**行内项目名**（与表单输入解耦）', async () => {
    const w = mountPanel({ rows: [project({ name: 'row-app' })], name: 'form-app' })
    const btns = w.findAll('.row-actions button')
    await btns[0].trigger('click')
    await btns[1].trigger('click')

    expect(w.emitted('up')?.[0]).toEqual(['row-app'])
    expect(w.emitted('down')?.[0]).toEqual(['row-app'])
  })

  it('空项目列表渲染空态（替身表格的 empty 分支）', () => {
    const w = mountPanel({ rows: [] })
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })
})

describe('ComposePanel · 转换入口与提示（M-D）', () => {
  it('工具栏「从 docker run 导入」上抛 import-run（不在部署按钮组内）', async () => {
    const w = mountPanel()
    await w.find('.compose-toolbar button').trigger('click')
    expect(w.emitted('import-run')).toBeTruthy()
    expect(w.findAll('.form-actions button')).toHaveLength(3)
  })

  it('有转换提示时逐条展示；无提示不渲染提示框', () => {
    expect(mountPanel().find('.convert-warnings').exists()).toBe(false)
    const w = mountPanel({ warnings: ['A 提示', 'B 提示'] })
    expect(w.findAll('.convert-warnings li').map((li) => li.text())).toEqual(['A 提示', 'B 提示'])
  })
})
