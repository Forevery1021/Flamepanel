import { describe, it, expect } from 'vitest'
import {
  testStatusTag,
  testStatusLabel,
  jobStatusTag,
  jobStatusLabel,
  jobScopeLabel,
  isRestorableKey,
  restoreScopeLabel,
  formatDateTime,
  accountTypeOptions,
  accountConfigPlaceholder,
  accountSelectOptions,
  accountNameOf,
} from '@/utils/backupFormat'
import type { BackupAccountDto, BackupJobDto } from '@/api/generated'

/**
 * T2-4 第四站拆出的备份域公共纯函数契约（原为 BackupView 内联实现）。
 *
 * 这里锁的不是「函数能跑」，而是七条**有产品含义的映射规则**：
 * ① 未测试不等于失败（不染红）；② 未知任务状态不猜成成功；
 * ③ app 范围必须带出是哪个应用；④ 可还原判据与后端受理范围同源（两道条件）；
 * ⑤ 时间缺失给 `-`（空白单元格会被读成加载失败）；⑥ 目标账号只列启用项；
 * ⑦ 账号被删后任务表回落 `#id`（暴露数据不一致，而不是渲染成空白）。
 */

const t = (key: string) => {
  const map: Record<string, string> = {
    'backup.testSuccess': '成功',
    'backup.testFailed': '失败',
    'backup.testNever': '未测试',
    'backup.jobStatusSuccess': '成功',
    'backup.jobStatusFailed': '失败',
    'backup.jobStatusNever': '从未执行',
    'backup.scopePanel': '面板数据库',
    'backup.scopeApp': '应用目录',
    'backup.accountTypeLocal': '本地/挂载盘',
  }
  return map[key] ?? key
}

const account = (over: Partial<BackupAccountDto> = {}): BackupAccountDto =>
  ({
    id: 1,
    name: 'oss',
    account_type: 's3',
    enabled: true,
    created_at: '',
    updated_at: '',
    last_test_at: null,
    last_test_status: null,
    ...over,
  }) as BackupAccountDto

const job = (over: Partial<BackupJobDto> = {}): BackupJobDto =>
  ({
    id: 1,
    name: 'nightly',
    scope: 'panel',
    scope_key: null,
    schedule: '0 3 * * *',
    target_account_id: 1,
    retention: 7,
    enabled: true,
    last_status: 'never',
    next_run_at: null,
    last_run_at: null,
    last_message: null,
    last_remote_key: null,
    created_at: '',
    updated_at: '',
    ...over,
  }) as BackupJobDto

describe('backupFormat · 账号测试状态映射', () => {
  it('success → success（绿）', () => {
    expect(testStatusTag('success')).toBe('success')
  })

  it('failed → danger（红）', () => {
    expect(testStatusTag('failed')).toBe('danger')
  })

  it('【关键】null（从未测试）→ neutral，不猜成失败', () => {
    expect(testStatusTag(null)).toBe('neutral')
    expect(testStatusTag('')).toBe('neutral')
  })

  it('文案三态：成功 / 失败 / 未测试', () => {
    expect(testStatusLabel('success', t)).toBe('成功')
    expect(testStatusLabel('failed', t)).toBe('失败')
    expect(testStatusLabel(null, t)).toBe('未测试')
  })
})

describe('backupFormat · 任务状态映射', () => {
  it('success / failed / never 三态各归其位', () => {
    expect(jobStatusTag('success')).toBe('success')
    expect(jobStatusTag('failed')).toBe('danger')
    expect(jobStatusTag('never')).toBe('neutral')
    expect(jobStatusLabel('success', t)).toBe('成功')
    expect(jobStatusLabel('failed', t)).toBe('失败')
    expect(jobStatusLabel('never', t)).toBe('从未执行')
  })

  it('【关键】未知状态值不猜成成功（后端可能新增 running 等中间态）', () => {
    expect(jobStatusTag('running')).toBe('neutral')
    expect(jobStatusLabel('running', t)).toBe('从未执行')
  })
})

describe('backupFormat · 任务范围文案', () => {
  it('panel 范围：只报「面板数据库」', () => {
    expect(jobScopeLabel(job({ scope: 'panel' }), t)).toBe('面板数据库')
  })

  it('app 范围：带出应用 key（不写是哪个应用等于让每行都写着「某处」）', () => {
    expect(jobScopeLabel(job({ scope: 'app', scope_key: 'blog' }), t)).toBe('应用目录: blog')
  })

  it('app 范围但 key 缺失：回落 `-`（不省略，省略会让行高随数据变化）', () => {
    expect(jobScopeLabel(job({ scope: 'app', scope_key: null }), t)).toBe('应用目录: -')
  })
})

describe('backupFormat · 可还原判据', () => {
  it('面板库 .db → 可还原', () => {
    expect(isRestorableKey('panel-20260914.db')).toBe(true)
  })

  it('应用数据 app/<key>/*.tar.gz → 可还原', () => {
    expect(isRestorableKey('app/blog/20260914.tar.gz')).toBe(true)
  })

  it('【关键】两道条件缺一不可：app/ 前缀下的非 tar.gz 不可还原', () => {
    expect(isRestorableKey('app/blog/notes.txt')).toBe(false)
    expect(isRestorableKey('app/blog/20260914.tar')).toBe(false)
  })

  it('其它任意对象（手工上传）不可还原', () => {
    expect(isRestorableKey('docs/readme.md')).toBe(false)
    expect(isRestorableKey('backup.db.bak')).toBe(false)
  })

  it('范围标签：app/ 前缀 → 应用目录，其余 → 面板数据库', () => {
    expect(restoreScopeLabel('app/blog/x.tar.gz', t)).toBe('应用目录')
    expect(restoreScopeLabel('panel.db', t)).toBe('面板数据库')
  })
})

describe('backupFormat · 时间与账号候选', () => {
  it('有时间 → 本地化串；null / undefined → `-`（空白会被读成加载失败）', () => {
    const iso = '2026-01-01T00:00:00Z'
    expect(formatDateTime(iso)).toBe(new Date(iso).toLocaleString())
    expect(formatDateTime(null)).toBe('-')
    expect(formatDateTime(undefined)).toBe('-')
  })

  it('账号类型候选：S3 / SFTP 不翻译，local 走译文', () => {
    expect(accountTypeOptions(t)).toEqual([
      { label: 'S3', value: 's3' },
      { label: 'SFTP', value: 'sftp' },
      { label: '本地/挂载盘', value: 'local' },
    ])
  })

  it('配置样例：三种类型各有模板，未知类型回落空串（不给错的模板）', () => {
    expect(accountConfigPlaceholder('s3')).toContain('"bucket"')
    expect(accountConfigPlaceholder('sftp')).toContain('"host"')
    expect(accountConfigPlaceholder('local')).toContain('"path"')
    expect(accountConfigPlaceholder('cos')).toBe('')
  })

  it('【关键】目标账号候选只列启用账号（停用账号建任务必然失败）', () => {
    const opts = accountSelectOptions([
      account({ id: 1, name: 'oss', account_type: 's3', enabled: true }),
      account({ id: 2, name: 'nas', account_type: 'local', enabled: false }),
    ])
    expect(opts).toHaveLength(1)
    expect(opts[0].value).toBe(1)
  })

  it('账号候选标签带类型后缀（同名不同型必须可分辨）', () => {
    const opts = accountSelectOptions([
      account({ id: 1, name: 'backup', account_type: 's3' }),
      account({ id: 2, name: 'backup', account_type: 'sftp' }),
    ])
    expect(opts.map((o) => o.label)).toEqual(['backup (S3)', 'backup (SFTP)'])
  })

  it('账号名查表：命中给名字，未命中回落 `#id`（暴露数据不一致）', () => {
    const list = [account({ id: 3, name: 'oss' })]
    expect(accountNameOf(list, 3)).toBe('oss')
    expect(accountNameOf(list, 9)).toBe('#9')
  })
})
