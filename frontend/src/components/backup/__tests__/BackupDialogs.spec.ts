import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import BackupAccountDialog from '@/components/backup/BackupAccountDialog.vue'
import BackupJobDialog from '@/components/backup/BackupJobDialog.vue'
import RestoreCloudDialog from '@/components/backup/RestoreCloudDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第四站拆出的三个弹窗契约（原为 BackupView 内联 FpModal）。
 *
 * 三个弹窗共享同一套「打开 → 校验 → 上抛 → 成功由父级关」的骨架，但各有
 * **不能丢的本地校验**——这些校验在拆分前是内联 `if (!x) { error = ...; return }`，
 * 最容易被「数据层也会校验」说服着删掉：但本地校验给你字段旁的红字，
 * 数据层的静默返回只会让用户以为「点了没反应」。
 *
 * 弹窗经 Teleport 到 body，断言一律落在 `document.body`（T1-1 已记录的陷阱）。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  backup: {
    accountCreate: '新建账号',
    accountEdit: '编辑账号',
    accountName: '账号名称',
    accountType: '类型',
    accountTypeLocal: '本地/挂载盘',
    accountConfig: '连接配置（JSON）',
    accountConfigEditHint: '编辑时留空表示不修改连接配置（敏感信息不回显）',
    accountEnabled: '启用',
    cloudRestore: '云端恢复',
    cloudRestoreLoad: '加载远端备份',
    cloudRestoreHint: '仅 .db 与 app/<key>/*.tar.gz 可还原',
    cloudRestoreKey: '远端对象',
    jobScope: '备份范围',
    jobScopeKey: '应用 Key',
    jobTarget: '目标账号',
    jobCreate: '新建任务',
    jobEdit: '编辑任务',
    jobName: '任务名称',
    jobSchedule: 'Cron 表达式',
    jobRetention: '保留份数',
    jobEnabled: '启用',
    scopePanel: '面板数据库',
    scopeApp: '应用目录',
    restore: '恢复',
  },
  common: {
    cancel: '取消',
    save: '保存',
    close: '关闭',
    required: '此项为必填项',
    colActions: '操作',
    noData: '暂无数据',
  },
}

function mountDialog(component: unknown, props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  return mountLoose(component, {
    props,
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
    },
  })
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

const accountRow = { id: 3, name: 'oss', account_type: 's3', enabled: true }

describe('BackupAccountDialog', () => {
  it('初始不渲染（visible=false）', async () => {
    mountDialog(BackupAccountDialog)
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('新建：标题为「新建账号」，字段给默认值（local / 启用）', async () => {
    const w = mountDialog(BackupAccountDialog)
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    expect(document.body.textContent).toContain('新建账号')
    const inputs = document.body.querySelectorAll('input')
    expect((inputs[0] as HTMLInputElement).value).toBe('')
  })

  it('编辑：标题为「编辑账号」且名称回填，配置框**刻意留空**（后端不回显）', async () => {
    const w = mountDialog(BackupAccountDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(accountRow)
    await settle()

    expect(document.body.textContent).toContain('编辑账号')
    const input = document.body.querySelector('input') as HTMLInputElement
    expect(input.value).toBe('oss')
    expect(document.body.querySelector('textarea')?.value).toBe('')
  })

  it('【关键】编辑态显示「留空表示不修改」提示（红字与提示互斥）', async () => {
    const w = mountDialog(BackupAccountDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(accountRow)
    await settle()
    expect(document.body.textContent).toContain('编辑时留空表示不修改连接配置（敏感信息不回显）')
  })

  it('【关键】新建 + 名称为空 → 本地拦下：字段旁红字且**不上抛**', async () => {
    const w = mountDialog(BackupAccountDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    expect(w.emitted('submit')).toBeFalsy()
    expect(document.body.textContent).toContain('此项为必填项')
  })

  it('【关键】编辑 + 配置留空 → 允许提交（空配置是「不修改」而不是错误）', async () => {
    const w = mountDialog(BackupAccountDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(accountRow)
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    const payload = w.emitted('submit')?.[0]?.[0] as Record<string, unknown>
    expect(payload).toMatchObject({ name: 'oss', account_type: 's3', config_json: '' })
  })

  it('新建 + 配置为空 → 拦下（新建必须给连接配置）', async () => {
    const w = mountDialog(BackupAccountDialog)
    const vm = w.vm as unknown as { open: () => void }
    vm.open()
    await settle()

    const input = document.body.querySelector('input') as HTMLInputElement
    input.value = 'new-acc'
    input.dispatchEvent(new Event('input'))
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    expect(w.emitted('submit')).toBeFalsy()
    expect(document.body.textContent).toContain('此项为必填项')
  })
})

describe('BackupJobDialog', () => {
  const jobRow = {
    id: 5,
    name: 'nightly',
    scope: 'panel',
    scope_key: null,
    schedule: '0 3 * * *',
    target_account_id: 1,
    retention: 7,
    enabled: true,
  }
  const accountOptions = [{ label: 'oss (S3)', value: 1 }]

  it('新建：标题「新建任务」，目标账号默认首个启用账号', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    expect(document.body.textContent).toContain('新建任务')
    const inputs = document.body.querySelectorAll('input')
    expect((inputs[0] as HTMLInputElement).value).toBe('')
    // 目标账号下拉的初值来自 accountOptions[0]
    expect(document.body.textContent).toContain('oss (S3)')
  })

  it('【关键】新建 + 无启用账号 → 目标账号校验拦下（本地红字，不换后端 400）', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions: [] })
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    expect(w.emitted('submit')).toBeFalsy()
    expect(document.body.textContent).toContain('此项为必填项')
  })

  it('编辑：标题「编辑任务」且名称、Cron 回填', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(jobRow)
    await settle()

    expect(document.body.textContent).toContain('编辑任务')
    const values = [...document.body.querySelectorAll('input')].map(
      (i) => (i as HTMLInputElement).value,
    )
    expect(values).toContain('nightly')
    expect(values).toContain('0 3 * * *')
  })

  it('编辑 + 完整表单 → 上抛七个字段（含 retention 数字）', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(jobRow)
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    const payload = w.emitted('submit')?.[0]?.[0] as Record<string, unknown>
    expect(payload).toMatchObject({
      name: 'nightly',
      scope: 'panel',
      schedule: '0 3 * * *',
      target_account_id: 1,
      retention: 7,
      enabled: true,
    })
  })

  it('【关键】scope=app 时才渲染「应用 Key」字段（摆着禁用会让人以为要填）', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    const vm = w.vm as unknown as { open: (r?: unknown) => void }
    vm.open()
    await settle()
    expect(document.body.textContent).not.toContain('应用 Key')

    ;(w.vm as unknown as { open: (r: unknown) => void }).open({
      ...jobRow,
      scope: 'app',
      scope_key: 'blog',
    })
    await settle()
    expect(document.body.textContent).toContain('应用 Key')
  })

  it('【关键】scope=app 且 key 为空 → 校验拦下', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open({
      ...jobRow,
      scope: 'app',
      scope_key: '',
    })
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    expect(w.emitted('submit')).toBeFalsy()
    expect(document.body.textContent).toContain('此项为必填项')
  })

  it('【关键】retention 只兜 null/undefined：后端的 0 原样保留、原样回传', async () => {
    // 后端约定 0 = 用默认 7；把它在 UI 里归一成 7 等于「用户没动过的字段被悄悄改掉」
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open({ ...jobRow, retention: 0 })
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    const payload = w.emitted('submit')?.[0]?.[0] as Record<string, unknown>
    expect(payload.retention).toBe(0)
  })

  it('retention 缺失（null）→ 兜 7', async () => {
    const w = mountDialog(BackupJobDialog, { accountOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open({ ...jobRow, retention: null })
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const save = buttons.find((b) => b.textContent?.includes('保存')) as HTMLButtonElement
    save.click()
    await settle()

    const payload = w.emitted('submit')?.[0]?.[0] as Record<string, unknown>
    expect(payload.retention).toBe(7)
  })
})

describe('RestoreCloudDialog', () => {
  const objects = [
    { key: 'panel-2026.db', size: 2048 },
    { key: 'app/blog/20260914.tar.gz', size: 4096 },
    { key: 'notes.txt', size: 12 },
  ]

  it('【关键】不可还原对象**照样列出**（用户需要看到桶里有什么），按钮禁用', async () => {
    const w = mountDialog(RestoreCloudDialog, {
      accountOptions: [{ label: 'oss (S3)', value: 1 }],
      objects,
      accountId: 1,
    })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    expect(document.body.textContent).toContain('notes.txt')
    expect(document.body.textContent).toContain('远端对象')
  })

  it('范围列：app/ 前缀 → 应用目录；其余 → 面板数据库', async () => {
    const w = mountDialog(RestoreCloudDialog, {
      accountOptions: [{ label: 'oss (S3)', value: 1 }],
      objects,
      accountId: 1,
    })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    expect(document.body.textContent).toContain('应用目录')
    expect(document.body.textContent).toContain('面板数据库')
  })

  it('「加载远端备份」按钮上抛 load（数据层按当前账号拉取）', async () => {
    const w = mountDialog(RestoreCloudDialog, {
      accountOptions: [{ label: 'oss (S3)', value: 1 }],
      objects: [],
      accountId: 1,
    })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const load = buttons.find((b) => b.textContent?.includes('加载远端备份')) as HTMLButtonElement
    load.click()
    await settle()
    expect(w.emitted('load')).toBeTruthy()
  })

  it('close()：关闭弹窗', async () => {
    const w = mountDialog(RestoreCloudDialog, {
      accountOptions: [{ label: 'oss (S3)', value: 1 }],
      objects: [],
      accountId: 1,
    })
    const vm = w.vm as unknown as { open: () => void; close: () => void }
    vm.open()
    await settle()
    expect(document.body.querySelector('.p-dialog')).not.toBeNull()

    vm.close()
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })
})
