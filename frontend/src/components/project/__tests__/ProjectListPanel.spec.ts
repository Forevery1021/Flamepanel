import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode, defineComponent } from 'vue'
import ProjectListPanel from '@/components/project/ProjectListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站拆出的项目列表契约（原为 ProjectView 内联表格）。
 *
 * **表格替身（TableStub）**：与 ContainerListPanel.spec / BackupJobPanel.spec 同一取舍——
 * 虚拟滚动依赖真实布局尺寸，而 jsdom 没有排版引擎，真表一行都不渲染。替身按
 * DataTable 的公开契约（`rows` + 各列 `body` 插槽逐行求值）复刻，单元格逻辑得到
 * 真实覆盖；纯对象写法（非 defineComponent）避免 `vue/one-component-per-file` 告警。
 *
 * 本表是全站操作最密的一张（**十类行操作**），且四条 `disabled` 条件都带业务语义。
 * 它们是本 spec 的重点：禁用条件写错不会报错，只会让用户点一个没反应的按钮。
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
  project: {
    name: '项目名称',
    kind: '项目类型',
    source: '来源',
    deployType: '部署方式',
    buildType: '构建器',
    port: '端口',
    domain: '域名',
    deploy: '部署',
    start: '启动',
    stop: '停止',
    restart: '重启',
    healthCheck: '健康检查',
    webhookEnable: 'Webhook',
    webhookReset: '重置令牌',
    logs: '日志',
    links: '数据库联链',
    kinds: { php: 'PHP 站点', node: 'Node.js', static: '静态站点', jar: 'Java JAR' },
    sources: { local_path: '本地目录', git: 'Git 仓库' },
    statuses: { building: '构建中', running: '运行中', stopped: '已停止', error: '异常' },
    deployTypes: { manual: '手动部署', webhook: 'Webhook' },
    buildTypes: { dockerfile: '容器构建', buildpacks: 'Buildpacks', runtime: '运行时' },
  },
  common: {
    noData: '暂无数据',
    status: '状态',
    colActions: '操作',
    edit: '编辑',
    delete: '删除',
  },
}

/**
 * 宽松挂载入口（与 ContainerListPanel.spec 同一取舍）：本 spec 刻意喂
 * 「部分字段」的行对象——真 DTO 的 15 个字段在这儿都是噪声，而精确签名会
 * 把每个用例绑死成 15 行样板。类型安全由组件侧 `defineProps<{ rows: ProjectDto[] }>`
 * 与生产调用点（ProjectView）保障。
 */
const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

/** 挂载 + 展开操作列（最后一列）里的全部按钮，供逐条断言 */
/** FpActionMenu 替身：把菜单项平铺成按钮，便于与行内按钮一起断言（真实弹层行为由 FpActionMenu.spec 锁） */
const ActionMenuStub = defineComponent({
  name: 'FpActionMenu',
  props: { items: { type: Array, default: () => [] } },
  template:
    '<button v-for="it in items" :key="it.key" :disabled="it.disabled || undefined" :class="[\'menu-item\', it.loading && \'p-button-loading\']" @click="it.command()">{{ it.label }}</button>',
})

function mountPanel(rows: Record<string, unknown>[], actingId = '') {
  document.body.innerHTML = ''
  return mountLoose(ProjectListPanel, {
    props: { rows, actingId },
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      // v-permission 走放行桩：权限可视性由 directives spec 单独锁，这里只看业务态
      directives: { permission: () => {} },
      stubs: {
        FpTable: TableStub,
        FpColumn: false,
        FpButton: false,
        FpTag: false,
        FpActionMenu: ActionMenuStub,
      },
    },
  })
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'blog',
  kind: 'node',
  source: 'git',
  deploy_type: 'manual',
  build_type: 'runtime',
  port: 3000,
  domain: 'blog.example.com',
  status: 'stopped',
  health_check_path: '/health',
  ...over,
})

/** 取某一行操作列内的按钮文本 → 按钮映射 */
function actionButtons(w: ReturnType<typeof mountPanel>, rowIndex = 0) {
  const cells = w.findAll(`.stub-row[data-p-index="${rowIndex}"] .stub-cell`)
  const actionCell = cells[cells.length - 1]
  type Btn = {
    text: () => string
    classes: () => string[]
    attributes: (n: string) => string | undefined
    trigger: (e: string) => Promise<unknown>
  }
  const list = actionCell.findAll('button') as unknown as Btn[]
  return {
    all: list,
    byText: (text: string) => list.find((b) => b.text() === text),
    texts: list.map((b) => b.text()),
  }
}

describe('ProjectListPanel · 列渲染', () => {
  it('空数据：渲染空态文案', () => {
    const w = mountPanel([])
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })

  it('字典列经纯函数翻译（类型 / 来源 / 状态），空 port 与 domain 显示 -', () => {
    const w = mountPanel([
      row({ kind: 'php', source: 'local_path', status: 'running', port: null, domain: null }),
    ])
    const text = w.find('.stub-row').text()
    expect(text).toContain('PHP 站点')
    expect(text).toContain('本地目录')
    expect(text).toContain('运行中')
    expect(text).toContain('-')
  })

  it('状态列走三态配色：running 绿 / building 黄 / error 红', () => {
    const w = mountPanel([
      row({ id: 1, status: 'running' }),
      row({ id: 2, status: 'building' }),
      row({ id: 3, status: 'error' }),
    ])
    const tags = w.findAll('.stub-row .stub-cell:nth-last-child(2) .fp-tag')
    expect(tags.map((t) => t.classes().join(' '))).toEqual([
      expect.stringContaining('success'),
      expect.stringContaining('warn'),
      expect.stringContaining('danger'),
    ])
  })

  it('webhook 部署与 dockerfile 构建器各带一个 info 标签（可辨识）', () => {
    const w = mountPanel([row({ deploy_type: 'webhook', build_type: 'dockerfile' })])
    expect(w.text()).toContain('Webhook')
    expect(w.text()).toContain('容器构建')
  })
})

describe('ProjectListPanel · 十类行操作', () => {
  it('十类操作齐备：3 个主按钮（部署/启动|停止/日志）+ 「更多」菜单 6 项', () => {
    const w = mountPanel([row({ status: 'stopped' })])
    expect(actionButtons(w).texts).toEqual([
      '部署',
      '启动',
      '日志',
      '重启',
      '健康检查',
      'Webhook',
      '数据库联链',
      '编辑',
      '删除',
    ])
    expect(actionButtons(mountPanel([row({ status: 'running' })])).texts.slice(0, 3)).toEqual([
      '部署',
      '停止',
      '日志',
    ])
  })

  it('每条操作上抛整行对象（不是 id——投影最容易漏改且改错不报错）', async () => {
    const w = mountPanel([row({ id: 7 })])
    const buttons = actionButtons(w)
    await buttons.byText('部署')!.trigger('click')
    await buttons.byText('日志')!.trigger('click')

    const emitted = w.emitted() as Record<string, unknown[][]>
    expect(emitted.deploy[0][0]).toMatchObject({ id: 7 })
    expect(emitted.logs[0][0]).toMatchObject({ id: 7 })
  })

  it('操作名与事件名一一对应（10 个事件都有出口）', async () => {
    // 两行：running 让「停止」可点，stopped 让「启动」可点——单行状态下两者必有一个禁用
    const w = mountPanel([
      row({ status: 'running' }),
      row({ id: 8, name: 'api', status: 'stopped' }),
    ])
    const b1 = actionButtons(w, 0)
    const b2 = actionButtons(w, 1)
    for (const label of [
      '部署',
      '停止',
      '重启',
      '健康检查',
      'Webhook',
      '日志',
      '数据库联链',
      '编辑',
      '删除',
    ]) {
      await b1.byText(label)!.trigger('click')
    }
    await b2.byText('启动')!.trigger('click')

    const names = Object.keys(w.emitted() as Record<string, unknown[]>)
    expect(names.filter((n) => n !== 'click').sort()).toEqual(
      [
        'deploy',
        'edit',
        'health',
        'links',
        'logs',
        'remove',
        'restart',
        'start',
        'stop',
        'webhook',
      ].sort(),
    )
  })
})

describe('ProjectListPanel · 四条禁用条件（状态机的一部分）', () => {
  it('【关键】运行中：只显示「停止」（启动不出现）', () => {
    const b = actionButtons(mountPanel([row({ status: 'running' })]))
    expect(b.byText('启动')).toBeFalsy()
    expect(b.byText('停止')!.attributes('disabled')).toBeUndefined()
  })

  it('【关键】未运行：只显示「启动」（停止不出现）', () => {
    const b = actionButtons(mountPanel([row({ status: 'stopped' })]))
    expect(b.byText('停止')).toBeFalsy()
    expect(b.byText('启动')!.attributes('disabled')).toBeUndefined()
  })

  it('【关键】未配健康检查路径：「健康检查」禁用（没有可探测的目标）', () => {
    const w = mountPanel([row({ health_check_path: '' })])
    expect(actionButtons(w).byText('健康检查')!.attributes('disabled')).toBeDefined()
  })

  it('【关键】本地目录来源：Webhook 禁用（没有可推送的仓库）', () => {
    const w = mountPanel([row({ source: 'local_path' })])
    expect(actionButtons(w).byText('Webhook')!.attributes('disabled')).toBeDefined()
  })

  it('Webhook 按钮文案随开关切换（重置令牌 / Webhook）', () => {
    const w = mountPanel([row({ deploy_type: 'webhook', source: 'git' })])
    expect(actionButtons(w).byText('重置令牌')).toBeTruthy()
    expect(actionButtons(w).byText('Webhook')).toBeFalsy()
  })
})

describe('ProjectListPanel · actingId 二维锁', () => {
  it('【关键】只锁当前行的当前按钮（同一行五个按钮不一起转圈）', () => {
    const w = mountPanel([row({ id: 9 }), row({ id: 8, name: 'api' })], 'deploy-9')
    const first = actionButtons(w, 0)
    const second = actionButtons(w, 1)

    expect(first.byText('部署')!.classes().join(' ')).toContain('p-button-loading')
    // 同一行的「重启」不转圈
    expect(first.byText('重启')!.classes().join(' ')).not.toContain('p-button-loading')
    // 别的行的「部署」不转圈
    expect(second.byText('部署')!.classes().join(' ')).not.toContain('p-button-loading')
  })

  it('健康探测与启动各自的标签互不串扰', () => {
    const w = mountPanel([row({ id: 4 })], 'health-4')
    const b = actionButtons(w)
    expect(b.byText('健康检查')!.classes().join(' ')).toContain('p-button-loading')
    expect(b.byText('启动')!.classes().join(' ')).not.toContain('p-button-loading')
  })
})
