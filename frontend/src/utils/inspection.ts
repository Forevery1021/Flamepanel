import type { Category, CheckStatus, Finding, InspectionReport, Severity } from '@/api/inspection'

/**
 * S-17 系统体检 — 纯函数：结论排序 / 分组 / 语义配色 / 文案 key 映射。
 *
 * 后端只给稳定 `code` + 语言无关 `evidence`；标题与建议在前端三语言包
 * `inspectionFinding.<code>.{title,advice}`（`locales/<lang>/inspectionFindings.ts`）中维护，未登记的 code 回退到通用文案。
 */

/** 后端 `Severity` 全集（降序：越靠前越严重） */
export const SEVERITY_ORDER: readonly Severity[] = ['critical', 'high', 'medium', 'low', 'info']

/** 分类展示顺序（与后端 `Category` 枚举一致，按"越底层越靠前"排） */
export const CATEGORY_ORDER: readonly Category[] = [
  'kernel',
  'cpu',
  'hardening',
  'access',
  'network',
  'updates',
  'storage',
  'panel',
]

/** 前端已登记标题 / 建议文案的 code（新增后端检查项时同步三语言包） */
export const KNOWN_FINDING_CODES = [
  'kernel_eol',
  'os_eol',
  'reboot_required',
  'mac_disabled',
  'cpu_vulnerability',
  'sysctl',
  'sshd_config',
  'sshd_root_login',
  'sshd_password_auth',
  'sshd_empty_passwords',
  'sshd_max_auth_tries',
  'sshd_default_port',
  'uid0_accounts',
  'empty_password_accounts',
  'shadow_permissions',
  'passwd_permissions',
  'docker_socket_permissions',
  'firewall_inactive',
  'exposed_service',
  'public_listeners',
  'pending_security_updates',
  'unattended_upgrades',
  'time_sync',
  'root_disk_usage',
  'swap_missing',
  'panel_session_timeout',
  'panel_metrics_token',
  'panel_process_user',
] as const
export type KnownFindingCode = (typeof KNOWN_FINDING_CODES)[number]

const KNOWN = new Set<string>(KNOWN_FINDING_CODES)

export function isKnownFindingCode(code: string): code is KnownFindingCode {
  return KNOWN.has(code)
}

/** 结论标题 / 建议的 i18n key；未登记的 code 回退到 `inspectionFinding.unknown.*` */
export function findingI18nKey(code: string, part: 'title' | 'advice'): string {
  return `inspectionFinding.${isKnownFindingCode(code) ? code : 'unknown'}.${part}`
}

export type Tone = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

/** 严重度 → FpTag 语义色 */
export function severityTone(severity: Severity): Tone {
  switch (severity) {
    case 'critical':
    case 'high':
      return 'danger'
    case 'medium':
      return 'warning'
    case 'low':
      return 'info'
    default:
      return 'neutral'
  }
}

/** 状态 → FpTag 语义色（fail 按严重度着色，pass 绿，unknown 灰） */
export function statusTone(finding: Pick<Finding, 'status' | 'severity'>): Tone {
  if (finding.status === 'pass') return 'success'
  if (finding.status === 'unknown') return 'neutral'
  return severityTone(finding.severity)
}

/** 评分 → 语义色：A/B 绿、C 黄、D/F 红 */
export function scoreTone(score: number): Tone {
  if (score >= 75) return 'success'
  if (score >= 60) return 'warning'
  return 'danger'
}

const STATUS_RANK: Record<CheckStatus, number> = { fail: 0, unknown: 1, pass: 2 }

/** 排序：fail 在前（按严重度降序），其次 unknown，最后 pass；同级按 code / subject 稳定排序 */
export function sortFindings(findings: readonly Finding[]): Finding[] {
  return [...findings].sort((a, b) => {
    const byStatus = STATUS_RANK[a.status] - STATUS_RANK[b.status]
    if (byStatus !== 0) return byStatus
    const bySeverity = SEVERITY_ORDER.indexOf(a.severity) - SEVERITY_ORDER.indexOf(b.severity)
    if (bySeverity !== 0) return bySeverity
    const byCode = a.code.localeCompare(b.code)
    if (byCode !== 0) return byCode
    return (a.subject ?? '').localeCompare(b.subject ?? '')
  })
}

export interface FindingGroup {
  category: Category
  findings: Finding[]
  failed: number
  unknown: number
}

/** 按分类分组（固定顺序，空分类省略），组内已排序 */
export function groupFindings(findings: readonly Finding[]): FindingGroup[] {
  const buckets = new Map<Category, Finding[]>()
  for (const f of findings) {
    const list = buckets.get(f.category)
    if (list) list.push(f)
    else buckets.set(f.category, [f])
  }
  return CATEGORY_ORDER.filter((c) => buckets.has(c)).map((category) => {
    const list = sortFindings(buckets.get(category) ?? [])
    return {
      category,
      findings: list,
      failed: list.filter((f) => f.status === 'fail').length,
      unknown: list.filter((f) => f.status === 'unknown').length,
    }
  })
}

/** 需要立刻处理的项：`fail` 且严重度 ≥ high，按严重度排序（仪表盘横幅 / 页顶摘要用） */
export function urgentFindings(report: Pick<InspectionReport, 'findings'>): Finding[] {
  return sortFindings(
    report.findings.filter(
      (f) => f.status === 'fail' && (f.severity === 'critical' || f.severity === 'high'),
    ),
  )
}

/** 运行时长 → `12d 3h` / `3h 5m` / `5m` */
export function formatUptime(secs: number): string {
  const days = Math.floor(secs / 86_400)
  const hours = Math.floor((secs % 86_400) / 3_600)
  const minutes = Math.floor((secs % 3_600) / 60)
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

/** 缓存年龄 → 分钟（向下取整），`0` 表示刚生成 */
export function cacheAgeMinutes(report: Pick<InspectionReport, 'cache_age_secs'>): number {
  return Math.floor(report.cache_age_secs / 60)
}
