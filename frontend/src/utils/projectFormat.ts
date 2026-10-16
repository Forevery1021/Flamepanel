/**
 * 项目域的展示映射与载荷组装纯函数（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 这些映射原先散落在 832 行的 ProjectView 内联实现里，被表格（类型/来源/状态三列）、
 * 创建编辑弹窗（三组下拉候选与运行时去重）、日志抽屉（日志行拼接）、
 * Webhook 弹窗（绝对地址拼接）四处共用。抽成纯函数后既可单测，
 * 也避免拆分子组件时「复制多份、改一处漏两处」——原文件里 `statusSeverity` / `kindLabel`
 * 都只写了一份，拆分时若各组件各抄一份就正好会踩这个坑。
 */

import type { LogLine, ProjectDto, RuntimeDto } from '@/api/generated'

/** 项目类型白名单（与后端 `Project::is_valid_kind` 对齐） */
export const PROJECT_KINDS = ['php', 'jar', 'war', 'node', 'python', 'go', 'static'] as const

/** 来源白名单：本地目录 / Git 仓库 */
export const PROJECT_SOURCES = ['local_path', 'git'] as const

/** 切换策略白名单：直接重启 / 零停机 rolling */
export const PROJECT_STRATEGIES = ['restart', 'rolling'] as const

/** FpTag 支持的语义色（与 FpTag 的 severity 取值同源，避免各调用点各写字面量） */
export type ProjectTagSeverity = 'success' | 'warning' | 'danger' | 'neutral'

/** 项目类型 → 文案（类型是后端标识符，译文按当前语言现算） */
export function kindLabel(kind: string, t: (key: string) => string): string {
  return t(`project.kinds.${kind}`)
}

/** 来源 → 文案 */
export function sourceLabel(source: string, t: (key: string) => string): string {
  return t(`project.sources.${source}`)
}

/** 运行状态 → 文案 */
export function statusLabel(status: string, t: (key: string) => string): string {
  return t(`project.statuses.${status}`)
}

/**
 * 运行状态 → 标签配色：running 绿 / building 黄 / error 红 / 其余中性。
 *
 * 未知状态**不猜**（回落 neutral 而非默认绿）：后端状态机新增取值时，
 * 把未知态显示成「运行中」是最容易误导运维的映射错误。
 * （本站由 832 行内的内联函数原样迁出，含 `stopped` 走中性这一既有口径。）
 */
export function statusSeverity(status: string): ProjectTagSeverity {
  if (status === 'running') return 'success'
  if (status === 'building') return 'warning'
  if (status === 'error') return 'danger'
  return 'neutral'
}

/**
 * 运行时下拉候选：**同 kind 只保留首个版本**，避免选项爆炸。
 *
 * 顺序即后端返回顺序（不做二次排序）——「默认版本排前面」是后端契约，
 * 前端再排一次反而会与之打架（原实现如此，拆分时原样保留）。
 */
export function runtimeOptions(runtimes: RuntimeDto[]): { label: string; value: number }[] {
  const used = new Set<string>()
  const out: { label: string; value: number }[] = []
  for (const r of runtimes) {
    if (used.has(r.kind)) continue
    used.add(r.kind)
    out.push({ label: `${r.kind} ${r.version}`, value: r.id })
  }
  return out
}

/** 类型下拉候选 */
export function kindSelectOptions(t: (key: string) => string) {
  return PROJECT_KINDS.map((k) => ({ label: kindLabel(k, t), value: k as string }))
}

/** 来源下拉候选 */
export function sourceSelectOptions(t: (key: string) => string) {
  return PROJECT_SOURCES.map((s) => ({ label: sourceLabel(s, t), value: s as string }))
}

/** 切换策略下拉候选 */
export function strategySelectOptions(t: (key: string) => string) {
  return PROJECT_STRATEGIES.map((s) => ({
    label: t(`project.strategies.${s}`),
    value: s as string,
  }))
}

/** 创建 / 编辑弹窗的表单态（编辑态由弹窗组件持有，提交时按此形状上抛） */
export interface ProjectFormState {
  name: string
  kind: string
  source: string
  source_url: string
  runtime_id: number | null
  run_command: string
  port: number | null
  domain: string
  health_check_path: string
  deploy_strategy: string
}

/** 新建表单初值（kind 默认 static、策略默认 restart——与后端默认语义一致） */
export function initProjectForm(): ProjectFormState {
  return {
    name: '',
    kind: 'static',
    source: 'local_path',
    source_url: '',
    runtime_id: null,
    run_command: '',
    port: null,
    domain: '',
    health_check_path: '',
    deploy_strategy: 'restart',
  }
}

/**
 * 由行数据回填编辑表单。
 *
 * 可空字段一律回落空串 / null（**不是**保持 undefined）：FpInput / FpSelect 的
 * model 类型不接受 undefined 之外的「缺失」，回填成 undefined 会让输入框在
 * 未触碰时也处于「无值」态，保存时再被 `|| null` 归一，两套空值语义互相打架。
 */
export function projectFormFrom(p: ProjectDto): ProjectFormState {
  return {
    name: p.name,
    kind: p.kind,
    source: p.source,
    source_url: p.source_url ?? '',
    runtime_id: p.runtime_id ?? null,
    run_command: p.run_command ?? '',
    port: p.port ?? null,
    domain: p.domain ?? '',
    health_check_path: p.health_check_path ?? '',
    deploy_strategy: p.deploy_strategy ?? 'restart',
  }
}

/**
 * 表单 → 创建/更新请求体的**公共字段**。
 *
 * 空串一律归一为 `null`：后端把空串当作「显式置空」与「未提供」的区分很弱，
 * 而 `domain=""` 这类值落到库表里会变成「有域名但为空」，运维排查时最容易被误导
 * （原实现的 `x || null` 口径，拆分时逐条保留）。
 */
export function buildProjectBody(form: ProjectFormState) {
  return {
    kind: form.kind,
    source: form.source,
    source_url: form.source_url || null,
    runtime_id: form.runtime_id,
    run_command: form.run_command || null,
    port: form.port,
    domain: form.domain || null,
    health_check_path: form.health_check_path || null,
    deploy_strategy: form.deploy_strategy,
  }
}

/** 表单 → 创建请求体（更新接口不收 name——项目名即部署目录，不可改名） */
export function buildProjectCreateBody(form: ProjectFormState) {
  return { name: form.name, ...buildProjectBody(form) }
}

/** 日志行 → 抽屉里的纯文本：有时间戳则前置 */
export function formatLogLines(lines: LogLine[]): string {
  return lines.map((l) => (l.timestamp ? `[${l.timestamp}] ${l.message}` : l.message)).join('\n')
}

/**
 * Webhook 绝对地址：后端回显的是 `/api/...` 相对路径，复制给 CI 配置必须补上
 * 面板自身的 origin（用户从内网地址配过去的 CI 才连得回来）。
 */
export function webhookAbsoluteUrl(origin: string, path: string): string {
  return `${origin}${path}`
}
