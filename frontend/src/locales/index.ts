import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'
import jaJP from './ja-JP'

const savedLang = localStorage.getItem('flame-lang') || 'zh-CN'

export const i18n = createI18n({
  legacy: false,
  locale: savedLang,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
    'ja-JP': jaJP,
  },
})

export function setLanguage(lang: string) {
  localStorage.setItem('flame-lang', lang)
  i18n.global.locale.value = lang as 'zh-CN' | 'en-US' | 'ja-JP'
}
