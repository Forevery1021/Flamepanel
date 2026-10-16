import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  toLangTag,
  pickLocalizedText,
  matchesLocalizedQuery,
  type LocalizedTextPair,
} from '@/utils/localeField'

/**
 * T0-4 本地化字段取值回归（Doc/23 §3.1 / 问题 P0-6）。
 *
 * 历史缺陷：商店视图直接绑定 `pkg.short_desc_zh`（后端双语字段的中文项），
 * en/ja 界面下描述仍是中文，i18n 在该模块实质失效；搜索也只匹配中文字段。
 *
 * 本 spec 覆盖取值链（当前语言 → 英文 → 中文）与双语搜索匹配，
 * 并附「视图不得直接渲染 *_zh 字段」的源码护栏。
 */

const pair = (zh: string, en?: string): LocalizedTextPair => ({ zh, en })

describe('T0-4 toLangTag（locale 归一化）', () => {
  it('zh 系 → zh', () => {
    expect(toLangTag('zh')).toBe('zh')
    expect(toLangTag('zh-CN')).toBe('zh')
    expect(toLangTag('zh-TW')).toBe('zh')
  })

  it('en / ja 系 → 主标签', () => {
    expect(toLangTag('en-US')).toBe('en')
    expect(toLangTag('en')).toBe('en')
    expect(toLangTag('ja-JP')).toBe('ja')
  })

  it('分隔符兼容下划线与大小写', () => {
    expect(toLangTag('EN_us')).toBe('en')
  })

  it('空值回落 zh（默认语言）', () => {
    expect(toLangTag('')).toBe('zh')
    expect(toLangTag(null)).toBe('zh')
    expect(toLangTag(undefined)).toBe('zh')
  })
})

describe('T0-4 pickLocalizedText（取值链：当前语言 → 英文 → 中文）', () => {
  it('zh 界面取中文', () => {
    expect(pickLocalizedDesc('zh-CN', pair('中文简介', 'English desc'))).toBe('中文简介')
  })

  it('en 界面取英文', () => {
    expect(pickLocalizedDesc('en-US', pair('中文简介', 'English desc'))).toBe('English desc')
  })

  it('ja 等无专属字段的语言回落中文（任务单约定：缺失回退 zh）', () => {
    expect(pickLocalizedDesc('ja-JP', pair('中文简介', 'English desc'))).toBe('中文简介')
  })

  it('zh 界面英文缺失 → 中文（自身）', () => {
    expect(pickLocalizedDesc('zh-CN', pair('中文简介'))).toBe('中文简介')
  })

  it('en 界面英文缺失 → 回落中文', () => {
    expect(pickLocalizedDesc('en-US', pair('中文简介'))).toBe('中文简介')
  })

  it('en 界面英文字段为空串/空白 → 回落中文（不渲染空卡片）', () => {
    expect(pickLocalizedDesc('en-US', pair('中文简介', ''))).toBe('中文简介')
    expect(pickLocalizedDesc('en-US', pair('中文简介', '   '))).toBe('中文简介')
  })

  it('zh 界面中文缺失 → 回落英文（历史数据的反向兜底）', () => {
    expect(pickLocalizedDesc('zh-CN', pair('', 'English desc'))).toBe('English desc')
  })

  it('两字段皆空 → 空串（调用方决定占位）', () => {
    expect(pickLocalizedDesc('en-US', pair(''))).toBe('')
    expect(pickLocalizedDesc('zh-CN', {})).toBe('')
  })

  it('null/undefined 字段不抛错', () => {
    expect(pickLocalizedDesc('en-US', { zh: null, en: null })).toBe('')
    expect(pickLocalizedDesc('en-US', { zh: undefined, en: undefined })).toBe('')
  })
})

describe('T0-4 matchesLocalizedQuery（搜索双语字段全覆盖）', () => {
  const pkg = {
    name: 'Gitea',
    key: 'gitea',
    zh: '轻量 Git 服务',
    en: 'Lightweight Git hosting',
  }

  it('中文关键词命中（zh 字段）', () => {
    expect(matchesLocalizedQuery(pkg, '轻量')).toBe(true)
  })

  it('英文关键词命中（en 字段）——历史实现只搜中文字段会漏', () => {
    expect(matchesLocalizedQuery(pkg, 'lightweight')).toBe(true)
    expect(matchesLocalizedQuery(pkg, 'hosting')).toBe(true)
  })

  it('包名与 key 仍可命中（沿用历史语义）', () => {
    expect(matchesLocalizedQuery(pkg, 'gitea')).toBe(true)
  })

  it('大小写不敏感', () => {
    expect(matchesLocalizedQuery(pkg, 'GITEA')).toBe(true)
    expect(matchesLocalizedQuery(pkg, 'LightWeight')).toBe(true)
  })

  it('空查询不过滤（返回 true）', () => {
    expect(matchesLocalizedQuery(pkg, '')).toBe(true)
    expect(matchesLocalizedQuery(pkg, '   ')).toBe(true)
  })

  it('无匹配返回 false', () => {
    expect(matchesLocalizedQuery(pkg, 'zzz-not-exist')).toBe(false)
  })

  it('字段缺失不抛错', () => {
    expect(matchesLocalizedQuery({}, 'x')).toBe(false)
    expect(matchesLocalizedQuery({ en: null, zh: null }, '')).toBe(true)
  })
})

describe('T0-4 源码护栏（视图不得直接渲染 *_zh 字段）', () => {
  // T2-4 拆分后：卡片描述渲染在 `components/appStore/AppStoreGrid.vue`，
  // 搜索匹配在 `composables/useAppStoreCatalog.ts`——护栏随之指向新落点，
  // 语义不变（不得直绑 `short_desc_zh`、取值与搜索必须同源）。
  const gridPath = resolve(import.meta.dirname, '../../components/appStore/AppStoreGrid.vue')
  const catalogPath = resolve(import.meta.dirname, '../../composables/useAppStoreCatalog.ts')

  it('【核心回归】应用卡片无 pkg.short_desc_zh 直绑，统一走 localizedDesc', () => {
    const source = readFileSync(gridPath, 'utf-8')
    // 模板直绑（`{{ pkg.short_desc_zh }}`）与搜索单字段匹配均为历史缺陷形态
    expect(source).not.toContain('{{ pkg.short_desc_zh }}')
    expect(source).not.toContain('p.short_desc_zh')
  })

  it('描述渲染与搜索匹配同源（AppStoreGrid + useAppStoreCatalog）', () => {
    const grid = readFileSync(gridPath, 'utf-8')
    expect(grid).toContain('useLocalizedDesc')
    expect(grid).toContain('localizedDesc(pkg)')

    const catalog = readFileSync(catalogPath, 'utf-8')
    expect(catalog).toContain('useLocalizedDesc')
    expect(catalog).toContain('matchesQuery(p, search.value)')
  })
})

/** 局部辅助：避免与 composable 命名冲突的直观写法 */
function pickLocalizedDesc(locale: string, p: LocalizedTextPair): string {
  return pickLocalizedText(p, locale)
}
