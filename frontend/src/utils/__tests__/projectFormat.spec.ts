import { describe, it, expect } from 'vitest'
import {
  PROJECT_KINDS,
  PROJECT_SOURCES,
  PROJECT_STRATEGIES,
  buildProjectBody,
  buildProjectCreateBody,
  formatLogLines,
  initProjectForm,
  kindLabel,
  kindSelectOptions,
  projectFormFrom,
  runtimeOptions,
  sourceLabel,
  sourceSelectOptions,
  statusLabel,
  statusSeverity,
  strategySelectOptions,
  webhookAbsoluteUrl,
} from '@/utils/projectFormat'

/**
 * T2-4 第六站拆出的纯函数契约（原为 ProjectView 832 行内的内联实现）。
 *
 * 这层的价值在于「改了也不报错、只会静默出错」的映射：状态配色、空值归一、
 * 运行时去重。它们在拆分前完全处在 `src/views/**` 覆盖率排除区里，
 * 既没人能脱离整页挂载去验证，也没有任何机器可读的锁定。
 */

const t = (key: string) => `T:${key}`

const runtime = (over: Record<string, unknown> = {}) => ({
  id: 1,
  kind: 'php',
  version: '8.2',
  status: 'installed',
  install_path: '/opt/php',
  is_default: false,
  created_at: '',
  updated_at: '',
  ...over,
})

const project = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'blog',
  kind: 'node',
  source: 'git',
  source_url: 'https://example.com/blog.git',
  runtime_id: 7,
  run_command: 'npm start',
  port: 3000,
  domain: 'blog.example.com',
  health_check_path: '/health',
  deploy_strategy: 'rolling',
  status: 'running',
  deploy_type: 'manual',
  build_type: 'dockerfile',
  deploy_path: '/opt/flamepanel/projects/blog',
  has_procfile: false,
  has_webhook_token: false,
  auto_deploy: false,
  created_at: '',
  updated_at: '',
  ...over,
})

describe('projectFormat · 字典映射', () => {
  it('类型 / 来源 / 状态三处文案走同一 key 规则', () => {
    expect(kindLabel('jar', t)).toBe('T:project.kinds.jar')
    expect(sourceLabel('local_path', t)).toBe('T:project.sources.local_path')
    expect(statusLabel('building', t)).toBe('T:project.statuses.building')
  })

  it('三组下拉候选与白名单一一对应（数量与顺序都不漂）', () => {
    expect(kindSelectOptions(t).map((o) => o.value)).toEqual([...PROJECT_KINDS])
    expect(sourceSelectOptions(t).map((o) => o.value)).toEqual([...PROJECT_SOURCES])
    expect(strategySelectOptions(t).map((o) => o.value)).toEqual([...PROJECT_STRATEGIES])
    expect(kindSelectOptions(t)[0]).toEqual({ label: 'T:project.kinds.php', value: 'php' })
    expect(strategySelectOptions(t)[0]).toEqual({
      label: 'T:project.strategies.restart',
      value: 'restart',
    })
  })
})

describe('projectFormat · 状态配色', () => {
  it('running 绿 / building 黄 / error 红', () => {
    expect(statusSeverity('running')).toBe('success')
    expect(statusSeverity('building')).toBe('warning')
    expect(statusSeverity('error')).toBe('danger')
  })

  it('【关键】stopped 与未知状态都是中性（未知态不猜成绿色）', () => {
    expect(statusSeverity('stopped')).toBe('neutral')
    expect(statusSeverity('starting')).toBe('neutral')
    expect(statusSeverity('')).toBe('neutral')
  })
})

describe('projectFormat · 运行时候选去重', () => {
  it('【关键】同 kind 只保留首个版本（选项爆炸防线）', () => {
    const list = [
      runtime({ id: 1, kind: 'php', version: '8.2', is_default: true }),
      runtime({ id: 2, kind: 'php', version: '8.1' }),
      runtime({ id: 3, kind: 'node', version: '20' }),
      runtime({ id: 4, kind: 'node', version: '18' }),
    ]
    expect(runtimeOptions(list as never)).toEqual([
      { label: 'php 8.2', value: 1 },
      { label: 'node 20', value: 3 },
    ])
  })

  it('不二次排序（默认版本排前是后端契约）', () => {
    const list = [
      runtime({ id: 9, kind: 'node', version: '20' }),
      runtime({ id: 1, kind: 'php', version: '8.2' }),
    ]
    expect(runtimeOptions(list as never).map((o) => o.value)).toEqual([9, 1])
  })

  it('空清单 → 空候选', () => {
    expect(runtimeOptions([])).toEqual([])
  })
})

describe('projectFormat · 表单初值与回填', () => {
  it('新建初值：static + local_path + restart（与后端默认语义一致）', () => {
    const form = initProjectForm()
    expect(form.kind).toBe('static')
    expect(form.source).toBe('local_path')
    expect(form.deploy_strategy).toBe('restart')
    expect(form.runtime_id).toBeNull()
    expect(form.port).toBeNull()
    expect(form.name).toBe('')
  })

  it('编辑回填：可空字段回落空串 / null，而不是留 undefined', () => {
    const form = projectFormFrom(
      project({
        source_url: null,
        runtime_id: null,
        run_command: null,
        port: null,
        domain: null,
        health_check_path: null,
        deploy_strategy: null,
      }) as never,
    )
    expect(form.source_url).toBe('')
    expect(form.runtime_id).toBeNull()
    expect(form.run_command).toBe('')
    expect(form.port).toBeNull()
    expect(form.domain).toBe('')
    expect(form.health_check_path).toBe('')
    // deploy_strategy 为 null 时回落 restart（后端默认值）
    expect(form.deploy_strategy).toBe('restart')
  })

  it('编辑回填：有值原样带出', () => {
    const form = projectFormFrom(project() as never)
    expect(form.name).toBe('blog')
    expect(form.source_url).toBe('https://example.com/blog.git')
    expect(form.runtime_id).toBe(7)
    expect(form.port).toBe(3000)
    expect(form.deploy_strategy).toBe('rolling')
  })
})

describe('projectFormat · 载荷组装', () => {
  it('【关键】空串一律归一为 null（空串落库会变成「有域名但为空」）', () => {
    const body = buildProjectBody({
      ...initProjectForm(),
      source_url: '',
      run_command: '',
      domain: '',
      health_check_path: '',
    })
    expect(body.source_url).toBeNull()
    expect(body.run_command).toBeNull()
    expect(body.domain).toBeNull()
    expect(body.health_check_path).toBeNull()
  })

  it('有值原样上报（含 0 端口这种「假值但合法」的边界）', () => {
    const body = buildProjectBody({ ...initProjectForm(), port: 0, domain: 'a.com' })
    expect(body.port).toBe(0)
    expect(body.domain).toBe('a.com')
  })

  it('创建体带 name，更新体不带（项目名即部署目录名，不可改）', () => {
    const form = { ...initProjectForm(), name: 'blog' }
    expect(buildProjectCreateBody(form)).toHaveProperty('name', 'blog')
    expect(buildProjectBody(form)).not.toHaveProperty('name')
  })
})

describe('projectFormat · 日志拼接与 Webhook 地址', () => {
  it('有时间戳前置，无时间戳直接输出（内建守护取不到时间的情形）', () => {
    expect(
      formatLogLines([
        { timestamp: '2026-09-15T10:00:00Z', message: 'started' },
        { timestamp: null, message: 'no-ts' },
      ] as never),
    ).toBe('[2026-09-15T10:00:00Z] started\nno-ts')
  })

  it('空列表 → 空串', () => {
    expect(formatLogLines([])).toBe('')
  })

  it('【关键】Webhook 绝对地址 = origin + 相对路径（配到 CI 里才连得回来）', () => {
    expect(
      webhookAbsoluteUrl('https://panel.example.com', '/api/projects/3/webhook?token=abc'),
    ).toBe('https://panel.example.com/api/projects/3/webhook?token=abc')
  })
})
