/**
 * 致命错误兜底文案（R-2，Doc/23 §2.5）。
 *
 * 与常规 UI 文案不同：错误边界可能**在 i18n 语言包尚未加载**（或 i18n 自身异常、
 * 组件树已损坏）时渲染兜底面板——此时若直接 `t('common.failed')`，界面上会显示
 * 裸 key（`common.failed`），甚至整块空白，等于兜底失效。
 *
 * 故此处内置三语言硬编码文案，作为「i18n 不可用」时的最后一道文案防线：
 * 语言优先级 = 用户已保存语言（localStorage）→ 浏览器语言 → 英文。
 *
 * 注意：语言包可用时仍优先走 i18n（见 composables/useSafeText），
 * 本文件的文案仅承担降级职责，键名与 `locales/*` 保持一致以便对照。
 */
import { STORAGE_KEYS } from './storage'

export type FallbackLang = 'zh' | 'en' | 'ja'

export interface FallbackTexts {
  /** 渲染错误标题（对照 i18n `common.renderError.title`） */
  renderTitle: string
  /** 刷新页面（对照 i18n `common.renderError.reload`） */
  reload: string
  /** 重试（对照 i18n `common.retry`） */
  retry: string
  /** 操作失败（对照 i18n `common.failed`） */
  failed: string
  /** 暂无数据（对照 i18n `common.noData`） */
  noData: string
}

const TEXTS: Record<FallbackLang, FallbackTexts> = {
  zh: {
    renderTitle: '界面渲染异常',
    reload: '刷新页面',
    retry: '重试',
    failed: '操作失败',
    noData: '暂无数据',
  },
  en: {
    renderTitle: 'UI rendering error',
    reload: 'Reload page',
    retry: 'Retry',
    failed: 'Operation failed',
    noData: 'No Data',
  },
  ja: {
    renderTitle: '画面の描画エラー',
    reload: 'ページを更新',
    retry: '再試行',
    failed: '操作失敗',
    noData: 'データがありません',
  },
}

/** 语言标签归一化：`zh-CN` / `zh-TW` → `zh`，未知语言回落英文 */
export function toFallbackLang(lang: string | null | undefined): FallbackLang {
  const value = (lang || '').toLowerCase()
  if (value.startsWith('zh')) return 'zh'
  if (value.startsWith('ja')) return 'ja'
  return 'en'
}

/** 解析当前兜底语言：已保存语言优先，其次浏览器语言 */
export function resolveFallbackLang(): FallbackLang {
  let stored = ''
  try {
    stored = localStorage.getItem(STORAGE_KEYS.lang) || ''
  } catch {
    // 隐私模式 / 存储被禁用：忽略，回落浏览器语言
  }
  if (stored) return toFallbackLang(stored)
  const nav = typeof navigator !== 'undefined' ? navigator.language : ''
  return toFallbackLang(nav)
}

/** 取当前语言的硬编码兜底文案 */
export function fallbackText(lang: FallbackLang = resolveFallbackLang()): FallbackTexts {
  return TEXTS[lang]
}
