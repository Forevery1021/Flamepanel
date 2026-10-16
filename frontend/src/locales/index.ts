import { createI18n } from 'vue-i18n'
import type { WritableComputedRef } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'
import type zhCN from './zh-CN'

export type AppLocale = 'zh-CN' | 'en-US' | 'ja-JP'

const DEFAULT_LOCALE: AppLocale = 'zh-CN'
// P6：改用 @vueuse/core useStorage 统一持久化
const langRef = useStorage<AppLocale>(STORAGE_KEYS.lang, DEFAULT_LOCALE, undefined, {
  serializer: rawStringSerializer<AppLocale>(),
})
const savedLang: AppLocale = langRef.value || DEFAULT_LOCALE

// 所有语言统一懒加载（含默认语言）：
// 静态引入 + 动态 import 混用会触发 Rollup INEFFECTIVE_DYNAMIC_IMPORT 警告，
// 且默认语言包失去分包收益；改为启动时异步装载默认语言，消除冗余打进首屏。
export const i18n = createI18n({
  legacy: false,
  locale: savedLang,
  fallbackLocale: DEFAULT_LOCALE,
  missingWarn: false,
  fallbackWarn: false,
  messages: {},
})

/**
 * 语言包加载 Promise（按语言去重）：启动期 `main.ts` 与路由守卫会并发请求同一语言，
 * 共用一个 in-flight Promise 只触发一次动态 import；失败时移除，允许下次重试。
 */
const localeLoads = new Map<AppLocale, Promise<void>>()

/**
 * 语言包懒加载映射（构建时自动分包为独立 chunk）。
 *
 * T2-5（Doc/23 §3.3）：语言包已从「一语言一个千行文件」拆为「一语言一个目录
 * + 按业务域分模块 + index.ts 聚合」，故此处路径显式写到目录下的 `index`
 * （与目录同名时省略 `/index` 也能解析，但显式书写可让「拆到目录后入口是谁」
 * 一眼可见）。聚合后的对外形状仍是 `typeof zhCN`（一份完整消息树），
 * 下游 `setLocaleMessage` / `useI18n` 调用方零改动。
 */
const localeLoaders: Record<AppLocale, () => Promise<{ default: typeof zhCN }>> = {
  'zh-CN': () => import('./zh-CN/index'),
  'en-US': () => import('./en-US/index'),
  'ja-JP': () => import('./ja-JP/index'),
}

/** 装载语言包（幂等、并发去重）：启动时对当前语言调用一次 */
export function loadLocaleMessages(locale: AppLocale): Promise<void> {
  let load = localeLoads.get(locale)
  if (!load) {
    load = localeLoaders[locale]().then((messages) => {
      i18n.global.setLocaleMessage(locale, messages.default as never)
    })
    load.catch(() => localeLoads.delete(locale))
    localeLoads.set(locale, load)
  }
  return load
}

/** 切换语言（自动懒加载对应语言包） */
export async function setLanguage(lang: string) {
  const locale = lang as AppLocale
  langRef.value = locale
  // 已加载则直接切换，否则先懒加载
  await loadLocaleMessages(locale)
  // B6：locale 通过 WritableComputedRef<AppLocale> 正确标注，避免无意义的强转掩盖类型问题
  ;(i18n.global.locale as unknown as WritableComputedRef<AppLocale>).value = locale
}

/**
 * 应用启动时装载保存的语言（含默认语言，均为异步分包加载）。
 *
 * 返回加载 Promise 供路由守卫 `await`——历史实现返回 `void`，守卫里的 `await` 形同虚设，
 * 首个导航可能在语言包就绪前放行（首屏短暂裸 key），单测里则表现为动态 import 在
 * 环境销毁后才落地（vitest `EnvironmentTeardownError`）。加载失败只告警不 reject：
 * 语言包缺席不应阻断导航（关键文案经 `useSafeText` 有硬编码兜底）。
 */
export function preloadSavedLocale(): Promise<void> {
  return setLanguage(savedLang).catch((err: unknown) => {
    console.warn('[i18n] preload locale failed:', savedLang, err)
  })
}
