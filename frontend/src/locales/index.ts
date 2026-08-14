import { createI18n } from 'vue-i18n'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'
import zhCN from './zh-CN'

export type AppLocale = 'zh-CN' | 'en-US' | 'ja-JP'

const DEFAULT_LOCALE: AppLocale = 'zh-CN'
// P6：改用 @vueuse/core useStorage 统一持久化
const langRef = useStorage<AppLocale>(STORAGE_KEYS.lang, DEFAULT_LOCALE, undefined, {
  serializer: rawStringSerializer<AppLocale>(),
})
const savedLang: AppLocale = langRef.value || DEFAULT_LOCALE

export const i18n = createI18n({
  legacy: false,
  locale: savedLang,
  fallbackLocale: DEFAULT_LOCALE,
  // 仅默认语言静态引入，其余语言按需懒加载
  messages: {
    'zh-CN': zhCN,
  },
})

/** 已加载的语言包集合（用于避免重复懒加载） */
const loadedLocales = new Set<AppLocale>(['zh-CN'])

/** 语言模块懒加载映射（构建时自动分包为独立 chunk） */
const localeLoaders: Record<AppLocale, () => Promise<{ default: typeof zhCN }>> = {
  'zh-CN': () => import('./zh-CN'),
  'en-US': () => import('./en-US'),
  'ja-JP': () => import('./ja-JP'),
}

/** 切换语言（自动懒加载对应语言包） */
export async function setLanguage(lang: string) {
  const locale = lang as AppLocale
  langRef.value = locale
  // 已加载则直接切换，否则先懒加载
  if (!loadedLocales.has(locale)) {
    const messages = await localeLoaders[locale]()
    i18n.global.setLocaleMessage(locale, messages.default as never)
    loadedLocales.add(locale)
  }
  i18n.global.locale.value = locale as 'zh-CN'
}

/** 应用启动时若保存语言非默认语言，异步加载对应语言包 */
export function preloadSavedLocale() {
  if (savedLang !== DEFAULT_LOCALE) {
    void setLanguage(savedLang)
  }
}
