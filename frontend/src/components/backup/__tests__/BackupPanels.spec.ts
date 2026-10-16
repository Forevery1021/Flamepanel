import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import LocalBackupPanel from '@/components/backup/LocalBackupPanel.vue'
import BackupAccountPanel from '@/components/backup/BackupAccountPanel.vue'
import BackupJobPanel from '@/components/backup/BackupJobPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站拆出的三张表契约（原为 BackupView 内联表格）。
 *
 * 与第三站相同的挂载难题：FpTable 在 jsdom 下虚拟滚动不渲染行，故用列替身
 * 把 `header / field / body` 三条契约显式重建——这样断言的是「列声明了什么」，
 * 而不是「某个底层库渲染成什么样」。
 *
 * 覆盖重点：三张表的**行操作载荷形状**（本地备份按 `filename`，账号/任务按整行）、
 * 开关两参数上抛、测试/执行 loading **只锁当前行**、任务表跨表译名（含账号被删回落）。
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
  backup: {
    filename: '备份文件',
    size: '大小',
    createdAt: '创建时间',
    download: '下载',
    restore: '恢复',
    delete: '删除',
    accountName: '账号名称',
    accountType: '类型',
    accountEnabled: '启用',
    accountTest: '测试连接',
    accountTestStatus: '测试结果',
    accountTestedAt: '测试时间',
    testSuccess: '成功',
    testFailed: '失败',
    testNever: '未测试',
    jobName: '任务名称',
    jobScope: '备份范围',
    jobSchedule: 'Cron 表达式',
    jobTarget: '目标账号',
    jobRetention: '保留份数',
    jobEnabled: '启用',
    jobLastStatus: '上次状态',
    jobNextRun: '下次执行',
    jobRun: '立即执行',
    jobStatusSuccess: '成功',
    jobStatusFailed: '失败',
    jobStatusNever: '从未执行',
    scopePanel: '面板数据库',
    scopeApp: '应用目录',
  },
  common: {
    noData: '暂无数据',
    colActions: '操作',
    edit: '编辑',
    delete: '删除',
  },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(component: unknown, props: Record<string, unknown> = {}) {
  return mountLoose(component, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
      stubs: { FpTable: TableStub },
    },
  })
}

function headers(w: ReturnType<typeof mountPanel>): string[] {
  return w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header') ?? '')
}

function actionLabels(w: ReturnType<typeof mountPanel>): string[] {
  return w.findAll('.row-actions button').map((b) => b.text())
}

const backup = (over: Record<string, unknown> = {}) => ({
  filename: 'panel-2026.db',
  size: 2048,
  created_at: '2026-01-01 03:00:00',
  ...over,
})

const account = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'oss',
  account_type: 's3',
  enabled: true,
  created_at: '',
  updated_at: '',
  last_test_at: '2026-01-01T00:00:00Z',
  last_test_status: 'success',
  ...over,
})

const job = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'nightly',
  scope: 'panel',
  scope_key: null,
  schedule: '0 3 * * *',
  target_account_id: 1,
  retention: 7,
  enabled: true,
  last_status: 'success',
  next_run_at: '2026-01-02T03:00:00Z',
  last_run_at: null,
  last_message: null,
  last_remote_key: null,
  created_at: '',
  updated_at: '',
  ...over,
})

describe('LocalBackupPanel · 列与行操作', () => {
  it('声明四列（备份文件/大小/创建时间/操作）', () => {
    const w = mountPanel(LocalBackupPanel, { rows: [backup()] })
    expect(headers(w)).toEqual(['备份文件', '大小', '创建时间', '操作'])
  })

  it('大小列走统一 formatSize（后端字节数不直出）', () => {
    const w = mountPanel(LocalBackupPanel, { rows: [backup({ size: 2048 })] })
    expect(w.find('[data-col="1"]').text()).toBe('2 KB')
  })

  it('三个行操作按钮：下载 / 恢复 / 删除', () => {
    const w = mountPanel(LocalBackupPanel, { rows: [backup()] })
    expect(actionLabels(w)).toEqual(['下载', '恢复', '删除'])
  })

  it('下载上抛**文件名**（不是整行——下载接口按名字寻址）', async () => {
    const w = mountPanel(LocalBackupPanel, { rows: [backup({ filename: 'x.db' })] })
    await w.findAll('.row-actions button')[0].trigger('click')
    expect(w.emitted('download')?.[0]).toEqual(['x.db'])
  })

  it('恢复 / 删除同样按文件名上抛', async () => {
    const w = mountPanel(LocalBackupPanel, { rows: [backup({ filename: 'x.db' })] })
    const buttons = w.findAll('.row-actions button')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')
    expect(w.emitted('restore')?.[0]).toEqual(['x.db'])
    expect(w.emitted('remove')?.[0]).toEqual(['x.db'])
  })

  it('空数据 → 空态文案由 FpTable 承载（列不渲染）', () => {
    const w = mountPanel(LocalBackupPanel, { rows: [] })
    expect(w.find('.stub-empty').text()).toBe('暂无数据')
  })
})

describe('BackupAccountPanel · 列与行操作', () => {
  it('声明六列（账号名称/类型/启用/测试结果/测试时间/操作）', () => {
    const w = mountPanel(BackupAccountPanel, { rows: [account()] })
    expect(headers(w)).toEqual(['账号名称', '类型', '启用', '测试结果', '测试时间', '操作'])
  })

  it('类型列：大写直出（S3 / SFTP 不翻译）', () => {
    const w = mountPanel(BackupAccountPanel, { rows: [account({ account_type: 'sftp' })] })
    expect(w.find('[data-col="1"]').text()).toBe('SFTP')
  })

  it('测试结果三态：成功 / 失败 / 未测试', () => {
    expect(
      mountPanel(BackupAccountPanel, { rows: [account({ last_test_status: 'success' })] })
        .find('[data-col="3"]')
        .text(),
    ).toBe('成功')
    expect(
      mountPanel(BackupAccountPanel, { rows: [account({ last_test_status: 'failed' })] })
        .find('[data-col="3"]')
        .text(),
    ).toBe('失败')
    expect(
      mountPanel(BackupAccountPanel, { rows: [account({ last_test_status: null })] })
        .find('[data-col="3"]')
        .text(),
    ).toBe('未测试')
  })

  it('测试时间：无值给 `-`（空白会被读成加载失败）', () => {
    const w = mountPanel(BackupAccountPanel, { rows: [account({ last_test_at: null })] })
    expect(w.find('[data-col="4"]').text()).toBe('-')
  })

  it('三个行操作：测试连接 / 编辑 / 删除，均上抛整行', async () => {
    const w = mountPanel(BackupAccountPanel, { rows: [account()] })
    expect(actionLabels(w)).toEqual(['测试连接', '编辑', '删除'])
    const buttons = w.findAll('.row-actions button')
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')
    expect(w.emitted('test')?.[0]?.[0]).toMatchObject({ id: 1 })
    expect(w.emitted('edit')?.[0]?.[0]).toMatchObject({ id: 1 })
    expect(w.emitted('remove')?.[0]?.[0]).toMatchObject({ id: 1 })
  })

  it('【关键】测试按钮 loading 只锁当前行（其余行的按钮不带 loading）', () => {
    const w = mountPanel(BackupAccountPanel, {
      rows: [account({ id: 1 }), account({ id: 2 })],
      testingId: 1,
    })
    // 每行 3 个按钮（测试/编辑/删除），仅第 1 行的「测试连接」带 loading
    const buttons = w.findAllComponents({ name: 'FpButton' })
    const loadings = buttons.map((b) => Boolean(b.props('loading')))
    expect(loadings).toEqual([true, false, false, false, false, false])
  })
})

describe('BackupJobPanel · 列与跨表译名', () => {
  it('声明九列（任务名称/范围/Cron/目标账号/保留份数/启用/上次状态/下次执行/操作）', () => {
    const w = mountPanel(BackupJobPanel, { rows: [job()] })
    expect(headers(w)).toEqual([
      '任务名称',
      '备份范围',
      'Cron 表达式',
      '目标账号',
      '保留份数',
      '启用',
      '上次状态',
      '下次执行',
      '操作',
    ])
  })

  it('范围列：panel 只报「面板数据库」；app 带出 key', () => {
    expect(mountPanel(BackupJobPanel, { rows: [job()] }).find('[data-col="1"]').text()).toBe(
      '面板数据库',
    )
    expect(
      mountPanel(BackupJobPanel, { rows: [job({ scope: 'app', scope_key: 'blog' })] })
        .find('[data-col="1"]')
        .text(),
    ).toBe('应用目录: blog')
  })

  it('【关键】目标账号走跨表译名：命中给名字（带类型后缀的候选不影响列表列）', () => {
    const w = mountPanel(BackupJobPanel, {
      rows: [job({ target_account_id: 1 })],
      accounts: [account({ id: 1, name: 'oss' })],
    })
    expect(w.find('[data-col="3"]').text()).toBe('oss')
  })

  it('【关键】账号被删后回落 `#id`（暴露数据不一致，而不是渲染空白）', () => {
    const w = mountPanel(BackupJobPanel, { rows: [job({ target_account_id: 9 })], accounts: [] })
    expect(w.find('[data-col="3"]').text()).toBe('#9')
  })

  it('上次状态三态映射：成功 / 失败 / 从未执行', () => {
    expect(
      mountPanel(BackupJobPanel, { rows: [job({ last_status: 'never' })] })
        .find('[data-col="6"]')
        .text(),
    ).toBe('从未执行')
    expect(
      mountPanel(BackupJobPanel, { rows: [job({ last_status: 'failed' })] })
        .find('[data-col="6"]')
        .text(),
    ).toBe('失败')
  })

  it('下次执行：无值给 `-`', () => {
    const w = mountPanel(BackupJobPanel, { rows: [job({ next_run_at: null })] })
    expect(w.find('[data-col="7"]').text()).toBe('-')
  })

  it('【关键】「立即执行」loading 只锁当前行', () => {
    const w = mountPanel(BackupJobPanel, { rows: [job({ id: 1 }), job({ id: 2 })], runningId: 2 })
    const loadings = w.findAllComponents({ name: 'FpButton' }).map((b) => Boolean(b.props('loading')))
    expect(loadings).toEqual([false, false, false, true, false, false])
  })

  it('三个行操作：立即执行 / 编辑 / 删除，均上抛整行', async () => {
    const w = mountPanel(BackupJobPanel, { rows: [job()] })
    expect(actionLabels(w)).toEqual(['立即执行', '编辑', '删除'])
    const buttons = w.findAll('.row-actions button')
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')
    await buttons[2].trigger('click')
    expect(w.emitted('run')?.[0]?.[0]).toMatchObject({ id: 1 })
    expect(w.emitted('edit')?.[0]?.[0]).toMatchObject({ id: 1 })
    expect(w.emitted('remove')?.[0]?.[0]).toMatchObject({ id: 1 })
  })
})

describe('三张表 · 开关上抛两参数', () => {
  it('账号表开关：上抛 (row, value)——数据层据此选启用/停用文案', async () => {
    const w = mountPanel(BackupAccountPanel, { rows: [account()] })
    const sw = w.findComponent({ name: 'FpSwitch' })
    sw.vm.$emit('update:modelValue', false)
    await w.vm.$nextTick()
    const payload = w.emitted('toggle')?.[0] as unknown[]
    expect(payload[0]).toMatchObject({ id: 1 })
    expect(payload[1]).toBe(false)
  })

  it('任务表开关：同样上抛 (row, value)', async () => {
    const w = mountPanel(BackupJobPanel, { rows: [job()] })
    const sw = w.findComponent({ name: 'FpSwitch' })
    sw.vm.$emit('update:modelValue', false)
    await w.vm.$nextTick()
    const payload = w.emitted('toggle')?.[0] as unknown[]
    expect(payload[0]).toMatchObject({ id: 1 })
    expect(payload[1]).toBe(false)
  })
})
