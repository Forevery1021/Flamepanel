import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { toFallbackLang, resolveFallbackLang, fallbackText } from '@/utils/fallbackText'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * R-2：致命错误兜底文案（不依赖 i18n 语言包）。
 * 语言优先级：已保存语言 → 浏览器语言 → 英文。
 */
describe('fallbackText 兜底文案（R-2）', () => {
  const original = navigator.language

  beforeEach(() => {
    localStorage.clear()
  })
  afterEach(() => {
    localStorage.clear()
    Object.defineProperty(navigator, 'language', { value: original, configurable: true })
  })

  it('语言标签归一化：zh-CN / zh-TW / zh → zh', () => {
    expect(toFallbackLang('zh-CN')).toBe('zh')
    expect(toFallbackLang('zh-TW')).toBe('zh')
    expect(toFallbackLang('zh')).toBe('zh')
  })

  it('语言标签归一化：ja / ja-JP → ja；未知 → en', () => {
    expect(toFallbackLang('ja-JP')).toBe('ja')
    expect(toFallbackLang('ja')).toBe('ja')
    expect(toFallbackLang('fr-FR')).toBe('en')
    expect(toFallbackLang('')).toBe('en')
    expect(toFallbackLang(null)).toBe('en')
    expect(toFallbackLang(undefined)).toBe('en')
  })

  it('已保存语言优先于浏览器语言', () => {
    localStorage.setItem(STORAGE_KEYS.lang, 'ja-JP')
    Object.defineProperty(navigator, 'language', { value: 'en-US', configurable: true })
    expect(resolveFallbackLang()).toBe('ja')
  })

  it('无保存语言时回落浏览器语言', () => {
    Object.defineProperty(navigator, 'language', { value: 'en-US', configurable: true })
    expect(resolveFallbackLang()).toBe('en')
  })

  it('三语言文案齐备且互不相同（无缺项/无占位）', () => {
    const keys = ['renderTitle', 'reload', 'retry', 'failed', 'noData'] as const
    const zh = fallbackText('zh')
    const en = fallbackText('en')
    const ja = fallbackText('ja')
    for (const k of keys) {
      for (const t of [zh, en, ja]) {
        expect(t[k]).toBeTruthy()
        expect(t[k]).not.toContain('undefined')
      }
      // 英文/日文不应与中文文案相同（防复制粘贴漏改）
      expect(en[k]).not.toBe(zh[k])
      expect(ja[k]).not.toBe(zh[k])
    }
  })
})
