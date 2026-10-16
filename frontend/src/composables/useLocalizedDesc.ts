import { useI18n } from 'vue-i18n'
import {
  pickLocalizedText,
  matchesLocalizedQuery,
  type LocalizedTextPair,
} from '@/utils/localeField'

/** 带双语描述字段的实体最小形态（应用商店 `AppMetadata` 结构兼容） */
export interface LocalizedPackage extends LocalizedTextPair {
  name?: string | null
  key?: string | null
  /** 后端中文字段名 */
  short_desc_zh?: string | null
  /** 后端英文字段名 */
  short_desc_en?: string | null
}

/**
 * 商店描述本地化取值（Doc/23 T0-4 / 问题 P0-6）。
 *
 * 视图直接用 `pkg.short_desc_zh` 会让 en/ja 界面显示中文；本 composable 把
 * 「按语言取双语字段」收敛为单点实现，取值规则见 `utils/localeField.ts`（兜底恒为中文）。
 *
 * **响应式**：返回的函数内部读取 `useI18n().locale`，在模板渲染或 `computed`
 * 中调用即自动订阅语言变更——切换语言后已渲染的卡片描述同步更新，无需重挂载。
 *
 * 用法：
 * ```vue
 * const { localizedDesc, matchesQuery } = useLocalizedDesc()
 * <div class="app-desc">{{ localizedDesc(pkg) }}</div>
 * const filtered = computed(() => list.value.filter((p) => matchesQuery(p, search.value)))
 * ```
 */
export function useLocalizedDesc() {
  const { locale } = useI18n()

  /** 按当前界面语言取描述文本 */
  function localizedDesc(pkg: LocalizedPackage | null | undefined): string {
    if (!pkg) return ''
    return pickLocalizedText({ zh: pkg.short_desc_zh, en: pkg.short_desc_en }, locale.value)
  }

  /** 搜索匹配（中英字段全覆盖，与显示语言解耦） */
  function matchesQuery(pkg: LocalizedPackage | null | undefined, query: string): boolean {
    if (!pkg) return false
    return matchesLocalizedQuery(
      {
        name: pkg.name,
        key: pkg.key,
        zh: pkg.short_desc_zh,
        en: pkg.short_desc_en,
      },
      query,
    )
  }

  return { localizedDesc, matchesQuery }
}
