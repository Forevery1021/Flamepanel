/**
 * 本地化字段取值（Doc/23 T0-4 / 问题 P0-6）。
 *
 * 应用商店等数据源的展示文本是**后端双语字段**（`short_desc_zh` / `short_desc_en`），
 * 不属于 i18n 词条——这类字段必须按当前界面语言挑选，而不是永远渲染 `*_zh`。
 * 历史实现直接绑定 `pkg.short_desc_zh`，en/ja 语言下商店描述仍是中文，
 * 该模块国际化实质失效；搜索也只用中文字段匹配。
 *
 * 取值链（空串/空白视为缺失）：
 * - **zh 界面**：中文 → 英文（历史/第三方源数据反向兜底）；
 * - **en 界面**：英文 → 中文（任务单约定「某语言缺失回退 zh」）；
 * - **其他语言（ja 等，后端无专属字段）**：回落中文（同上约定），中文亦空时用英文兜底。
 *
 * 本模块为**纯函数**（不依赖 vue / i18n），响应式包装见
 * `composables/useLocalizedDesc.ts`（视图统一经它取值）。
 */

/** 双语文本对（对应后端 `*_zh` / `*_en` 字段形态） */
export interface LocalizedTextPair {
  /** 中文文本（后端必填字段，作为最终兜底） */
  zh?: string | null
  /** 英文文本（后端可选字段，缺失时回落中文） */
  en?: string | null
}

/** 归一化 locale → 主语言标签（`en-US` / `en_US` → `en`；空值按 `zh` 处理） */
export function toLangTag(locale: string | null | undefined): string {
  const value = (locale || '').toLowerCase().trim()
  if (!value) return 'zh'
  return value.split(/[-_]/)[0] || 'zh'
}

/**
 * 按当前语言挑选本地化文本。
 *
 * 命中取值链的首个非空值；全空返回空串（调用方自行决定是否展示占位）。
 * 兜底方向恒为中文：`short_desc_zh` 是后端必填字段，其余语言字段可选，
 * 因此 en/ja 界面在缺英文时必须回落中文而非渲染空卡片。
 */
export function pickLocalizedText(
  pair: LocalizedTextPair,
  locale: string | null | undefined,
): string {
  const zh = (pair.zh ?? '').trim()
  const en = (pair.en ?? '').trim()
  // en 界面：英文优先；其余语言（zh / ja / …）：中文优先
  return toLangTag(locale) === 'en' ? en || zh : zh || en
}

/**
 * 搜索匹配：**同时覆盖中英两个字段**（与当前显示语言解耦）。
 *
 * 「英文关键词可命中」与「中文关键词可命中」都必须成立——用户搜索时不会关心
 * 当前界面语言对应的是哪个字段。同时匹配包名与 key（沿用历史搜索语义）。
 * 查询串为空视为不过滤（返回 true）。
 */
export function matchesLocalizedQuery(
  pkg: { name?: string | null; key?: string | null } & LocalizedTextPair,
  query: string,
): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  return [pkg.name, pkg.key, pkg.zh, pkg.en].some((field) =>
    (field ?? '').toLowerCase().includes(q),
  )
}
