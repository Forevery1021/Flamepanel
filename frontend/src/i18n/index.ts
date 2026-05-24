import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN'
import enUS from './locales/en-US'

const STORAGE_KEY = 'flamepanel_language'

function getSavedLocale(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) || 'zh-CN'
  } catch {
    return 'zh-CN'
  }
}

export const i18n = createI18n({
  legacy: false,
  locale: getSavedLocale(),
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})

export function setLocale(locale: string) {
  ;(i18n.global.locale as any).value = locale
  try {
    localStorage.setItem(STORAGE_KEY, locale)
  } catch { /* ignore */ }
}

export function getLocale(): string {
  return (i18n.global.locale as any).value || 'zh-CN'
}
