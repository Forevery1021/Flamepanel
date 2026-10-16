import { describe, expect, it } from 'vitest'
import type { Finding } from '@/api/inspection'
import {
  CATEGORY_ORDER,
  KNOWN_FINDING_CODES,
  cacheAgeMinutes,
  findingI18nKey,
  formatUptime,
  groupFindings,
  isKnownFindingCode,
  scoreTone,
  severityTone,
  sortFindings,
  statusTone,
  urgentFindings,
} from '@/utils/inspection'

/**
 * S-17 体检纯函数：排序 / 分组 / 语义配色 / 文案 key 回退。
 * 这些函数决定页面与横幅"先看到什么"，故逐条锁定顺序与阈值。
 */

function f(over: Partial<Finding> & Pick<Finding, 'code'>): Finding {
  return {
    subject: null as unknown as Finding['subject'],
    category: 'access',
    severity: 'medium',
    status: 'fail',
    evidence: '',
    heuristic: false,
    ...over,
  }
}

describe('utils/inspection · 文案 key', () => {
  it('已登记 code 映射到自身，未登记回退到 unknown', () => {
    expect(isKnownFindingCode('sshd_root_login')).toBe(true)
    expect(isKnownFindingCode('brand_new_check')).toBe(false)
    expect(findingI18nKey('sshd_root_login', 'title')).toBe(
      'inspectionFinding.sshd_root_login.title',
    )
    expect(findingI18nKey('brand_new_check', 'advice')).toBe('inspectionFinding.unknown.advice')
  })

  it('登记表无重复且分类顺序覆盖后端全部枚举', () => {
    expect(new Set(KNOWN_FINDING_CODES).size).toBe(KNOWN_FINDING_CODES.length)
    expect([...CATEGORY_ORDER].sort()).toEqual(
      ['kernel', 'cpu', 'hardening', 'access', 'network', 'updates', 'storage', 'panel'].sort(),
    )
  })
})

describe('utils/inspection · 语义配色', () => {
  it('严重度：critical/high 红、medium 黄、low 蓝、info 灰', () => {
    expect(severityTone('critical')).toBe('danger')
    expect(severityTone('high')).toBe('danger')
    expect(severityTone('medium')).toBe('warning')
    expect(severityTone('low')).toBe('info')
    expect(severityTone('info')).toBe('neutral')
  })

  it('状态：pass 绿、unknown 灰、fail 按严重度', () => {
    expect(statusTone({ status: 'pass', severity: 'critical' })).toBe('success')
    expect(statusTone({ status: 'unknown', severity: 'critical' })).toBe('neutral')
    expect(statusTone({ status: 'fail', severity: 'low' })).toBe('info')
    expect(statusTone({ status: 'fail', severity: 'high' })).toBe('danger')
  })

  it('评分阈值与后端等级一致：≥ 75 绿（A/B）、≥ 60 黄（C）、否则红', () => {
    expect(scoreTone(100)).toBe('success')
    expect(scoreTone(75)).toBe('success')
    expect(scoreTone(74)).toBe('warning')
    expect(scoreTone(60)).toBe('warning')
    expect(scoreTone(59)).toBe('danger')
  })
})

describe('utils/inspection · 排序与分组', () => {
  const findings: Finding[] = [
    f({
      code: 'sysctl',
      subject: 'kernel.kptr_restrict',
      category: 'hardening',
      severity: 'low',
      status: 'pass',
    }),
    f({ code: 'time_sync', category: 'updates', severity: 'medium', status: 'unknown' }),
    f({ code: 'sshd_root_login', category: 'access', severity: 'high' }),
    f({ code: 'uid0_accounts', category: 'access', severity: 'critical' }),
    f({ code: 'exposed_service', subject: '6379/tcp', category: 'network', severity: 'critical' }),
    f({ code: 'exposed_service', subject: '3306/tcp', category: 'network', severity: 'high' }),
    f({
      code: 'kernel_eol',
      category: 'kernel',
      severity: 'medium',
      status: 'pass',
      heuristic: true,
    }),
  ]

  it('fail 在前按严重度降序，其次 unknown，最后 pass；同级按 code / subject 稳定', () => {
    const order = sortFindings(findings).map((x) => `${x.code}${x.subject ? ':' + x.subject : ''}`)
    expect(order).toEqual([
      'exposed_service:6379/tcp',
      'uid0_accounts',
      'exposed_service:3306/tcp',
      'sshd_root_login',
      'time_sync',
      'kernel_eol',
      'sysctl:kernel.kptr_restrict',
    ])
    // 不改动入参
    expect(findings[0].code).toBe('sysctl')
  })

  it('按固定分类顺序分组，省略空分类，统计 failed / unknown', () => {
    const groups = groupFindings(findings)
    expect(groups.map((g) => g.category)).toEqual([
      'kernel',
      'hardening',
      'access',
      'network',
      'updates',
    ])
    const access = groups.find((g) => g.category === 'access')!
    expect(access.failed).toBe(2)
    expect(access.unknown).toBe(0)
    expect(access.findings.map((x) => x.code)).toEqual(['uid0_accounts', 'sshd_root_login'])
    const updates = groups.find((g) => g.category === 'updates')!
    expect(updates.failed).toBe(0)
    expect(updates.unknown).toBe(1)
  })

  it('urgentFindings 只取 fail 且 ≥ high，按严重度排序', () => {
    const urgent = urgentFindings({ findings }).map((x) => `${x.code}:${x.severity}`)
    expect(urgent).toEqual([
      'exposed_service:critical',
      'uid0_accounts:critical',
      'exposed_service:high',
      'sshd_root_login:high',
    ])
    expect(urgentFindings({ findings: [] })).toEqual([])
  })
})

describe('utils/inspection · 格式化', () => {
  it('运行时长分段：天+小时 / 小时+分钟 / 分钟', () => {
    expect(formatUptime(3 * 86_400 + 5 * 3_600 + 7 * 60)).toBe('3d 5h')
    expect(formatUptime(5 * 3_600 + 7 * 60 + 9)).toBe('5h 7m')
    expect(formatUptime(7 * 60 + 59)).toBe('7m')
    expect(formatUptime(0)).toBe('0m')
  })

  it('缓存年龄向下取整到分钟', () => {
    expect(cacheAgeMinutes({ cache_age_secs: 0 })).toBe(0)
    expect(cacheAgeMinutes({ cache_age_secs: 59 })).toBe(0)
    expect(cacheAgeMinutes({ cache_age_secs: 601 })).toBe(10)
  })
})
