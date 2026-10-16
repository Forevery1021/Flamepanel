import { useI18n } from 'vue-i18n'

/**
 * R-2：i18n 安全的取词函数（兜底面板专用）。
 *
 * 错误边界渲染时语言包可能尚未加载（启动期异步懒加载），或 i18n 实例本身已不可用。
 * 此场景下：
 * - `useI18n()` 在组件 setup 外调用会抛错 → 捕获后直接走硬编码兜底；
 * - 语言包未含该 key 时 `t()` 会返回裸 key（`common.failed`）→ 先 `te()` 探测再取词。
 *
 * 用法：`const tt = useSafeText(); tt('common.retry', '重试')`
 *
 * 说明：返回值类型刻意收窄为 `(key, fallback) => string`——`useI18n()` 的具体
 * Composer 泛型随 i18n 实例推断，收窄可避免调用方被其复杂签名污染。
 */

/** 取词所需的最小子集（结构化类型，不依赖 vue-i18n 的具体泛型） */
interface MinimalComposer {
  t: (key: string) => string
  te: (key: string) => boolean
}

export function useSafeText(): (key: string, fallback: string) => string {
  let composer: MinimalComposer | null = null
  try {
    composer = useI18n() as unknown as MinimalComposer
  } catch {
    // 无 setup 上下文 / i18n 未安装：全部走调用方提供的兜底文案
    composer = null
  }

  return function safeText(key: string, fallback: string): string {
    if (!composer) return fallback
    try {
      return composer.te(key) ? composer.t(key) : fallback
    } catch {
      return fallback
    }
  }
}
