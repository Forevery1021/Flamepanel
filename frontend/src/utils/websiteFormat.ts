/**
 * 网站 / 证书展示格式化（Doc/23 §3.3 T2-4 视图拆分的公共纯函数）。
 *
 * 这些映射原先散落在 1181 行的 WebsitesView 内联实现里，被证书表格（域名与到期
 * 时间列、状态标签）、绑定弹窗、伪静态弹窗三处共用；抽成纯函数后既可单测，
 * 也避免拆分子组件时「复制多份、改一处漏两处」。
 */

import { SITE_TYPES, REWRITE_PRESETS, type SiteType } from '@/api/websites'

/**
 * SAN JSON 字符串 → 逗号分隔域名。
 *
 * 后端 `Certificate.domains` 存的是 JSON 数组字符串；解析失败时**原样透出**
 * （历史库/手工改库可能存在非 JSON 值，此时展示原文比抛错更有用）。
 */
export function formatDomains(domains: string): string {
  try {
    return (JSON.parse(domains) as string[]).join(', ')
  } catch {
    return domains
  }
}

/**
 * 到期时间：空值 → 「未签发」占位文案（由调用方传 i18n 译文，纯函数不依赖 i18n），
 * 否则按浏览器本地时区格式化。
 */
export function formatDateTime(v: string | null | undefined, neverText: string): string {
  if (!v) return neverText
  return new Date(v).toLocaleString()
}

/** 证书三态（表格标签据此配色：过期 / 30 天内到期 / 有效） */
export type CertState = 'expired' | 'expiring' | 'valid'

/**
 * 证书状态判定：与后端 `needs_renewal`（30 天续签窗口）对齐。
 *
 * `now` 可注入，便于单测锁死时间边界（默认取当前时刻，运行期行为不变）。
 */
export function certState(c: { expires_at?: string | null }, now: number = Date.now()): CertState {
  if (!c.expires_at) return 'valid'
  const days = (new Date(c.expires_at).getTime() - now) / 86400000
  if (days < 0) return 'expired'
  if (days <= 30) return 'expiring'
  return 'valid'
}

/** 站点类型下拉项（`SITE_TYPES` 为权威清单，文案按当前语言现算） */
export function siteTypeOptions(t: (key: string) => string): { label: string; value: SiteType }[] {
  return SITE_TYPES.map((tp) => ({ label: t(`website.siteType_${tp}`), value: tp }))
}

/**
 * 伪静态模板候选（本地常量版）。
 *
 * 与接口 `supported` 的区别：**接口是模板库的单一事实来源**，本地常量用于两处
 * 接口未覆盖的场景——① 绑定弹窗的可选追加项（不依赖该站点是否已有套用状态）；
 * ② 伪静态弹窗在状态未加载完成时的兜底。两处都不做「模板名 → 译文」映射：
 * 模板名是后端标识符（wordpress / laravel…），翻译它反而对不上文档。
 */
export function rewritePresetOptions(): { label: string; value: string }[] {
  return REWRITE_PRESETS.map((pr) => ({ label: pr, value: pr }))
}

/** 站点状态筛选项（列表工具栏的客户端过滤） */
export function siteStatusOptions(t: (key: string) => string) {
  return [
    { label: t('website.active'), value: 'active' },
    { label: t('website.inactive'), value: 'inactive' },
  ]
}
