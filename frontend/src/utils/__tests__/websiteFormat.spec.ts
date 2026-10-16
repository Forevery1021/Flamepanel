import { describe, it, expect } from 'vitest'
import {
  certState,
  formatDateTime,
  formatDomains,
  rewritePresetOptions,
  siteStatusOptions,
  siteTypeOptions,
} from '@/utils/websiteFormat'
import { SITE_TYPES, REWRITE_PRESETS } from '@/api/websites'

/**
 * T2-4 拆分出的网站/证书展示纯函数（`utils/websiteFormat.ts`）。
 *
 * 这些映射原先是 WebsitesView 的内联函数，被证书表格（域名/到期列、状态标签）、
 * 绑定弹窗、伪静态弹窗三处消费——既不可单测，又会「改一处漏两处」。本 spec 锁住
 * 四件事：① 域名 JSON 解析与容错；② 空到期时间的占位；③ 证书三态的时间边界；
 * ④ 下拉候选与后端白名单常量同源（模板库/站点类型清单漂移必须红）。
 */

describe('websiteFormat · formatDomains', () => {
  it('JSON 数组 → 逗号分隔（SAN 列表的展示口径）', () => {
    expect(formatDomains('["example.com","*.example.com"]')).toBe('example.com, *.example.com')
  })

  it('单元素数组正常输出', () => {
    expect(formatDomains('["a.com"]')).toBe('a.com')
  })

  it('非 JSON 值原样透出（历史库/手工改库不得抛错白屏）', () => {
    expect(formatDomains('example.com')).toBe('example.com')
  })

  it('空字符串原样返回（不产生 "null"/"undefined" 文案）', () => {
    expect(formatDomains('')).toBe('')
  })
})

describe('websiteFormat · formatDateTime', () => {
  it('空值 → 调用方传入的占位文案（未签发/从未续签由 i18n 决定，纯函数不内嵌文案）', () => {
    expect(formatDateTime(null, '—')).toBe('—')
    expect(formatDateTime(undefined, '—')).toBe('—')
  })

  it('有值 → 本地化时间字符串（与 toLocaleString 一致）', () => {
    const iso = '2026-01-02T03:04:05Z'
    expect(formatDateTime(iso, '—')).toBe(new Date(iso).toLocaleString())
  })
})

describe('websiteFormat · certState（与后端 needs_renewal 30 天窗口同源）', () => {
  const now = Date.parse('2026-06-01T00:00:00Z')
  const at = (days: number) => new Date(now + days * 86400000).toISOString()

  it('无到期时间 → valid（未签发/待上传，不算过期）', () => {
    expect(certState({ expires_at: null }, now)).toBe('valid')
    expect(certState({}, now)).toBe('valid')
  })

  it('已过期（负天数）→ expired', () => {
    expect(certState({ expires_at: at(-1) }, now)).toBe('expired')
  })

  it('30 天整 → expiring（边界含端点，与续签窗口一致）', () => {
    expect(certState({ expires_at: at(30) }, now)).toBe('expiring')
  })

  it('30 天零 1 分 → valid（窗口外不误报）', () => {
    expect(certState({ expires_at: at(30.1) }, now)).toBe('valid')
  })

  it('7 天内仍走 expiring 档（不新增第三档；升级告警由后端事件负责）', () => {
    expect(certState({ expires_at: at(3) }, now)).toBe('expiring')
  })
})

describe('websiteFormat · 下拉候选', () => {
  const messages: Record<string, string> = {
    'website.siteType_runtime_php': 'PHP 运行时',
    'website.siteType_reverse_proxy': '反向代理',
    'website.siteType_static': '静态',
    'website.siteType_container_app': '容器应用',
    'website.active': '运行中',
    'website.inactive': '已停用',
  }
  const t = (k: string) => messages[k] ?? k

  it('站点类型候选与 SITE_TYPES 常量逐项同源（白名单漂移必须红）', () => {
    const opts = siteTypeOptions(t)
    expect(opts.map((o) => o.value)).toEqual([...SITE_TYPES])
    expect(opts.map((o) => o.label)).toEqual(['PHP 运行时', '反向代理', '静态', '容器应用'])
  })

  it('伪静态候选与 REWRITE_PRESETS 同源；模板名不做翻译（后端标识符）', () => {
    const opts = rewritePresetOptions()
    expect(opts.map((o) => o.value)).toEqual([...REWRITE_PRESETS])
    expect(opts.every((o) => o.label === o.value)).toBe(true)
  })

  it('状态筛选项为 active / inactive 两项', () => {
    expect(siteStatusOptions(t)).toEqual([
      { label: '运行中', value: 'active' },
      { label: '已停用', value: 'inactive' },
    ])
  })
})
