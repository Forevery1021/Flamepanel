/**
 * 应用商店展示格式化（Doc/23 §3.3 T2-4 视图拆分的公共纯函数）。
 *
 * 这些映射原先散落在 AppStoreView 内联实现，被卡片网格（模式标签）、已安装表、
 * 安装弹窗（模式下拉）三处共用；抽成纯函数后既可单测，也避免拆分子组件时
 * 「复制三份、改一处漏两处」。
 */

/** 应用包格式标签：flame / onepanel / baota → 品牌名（未知格式原样透出） */
export function formatLabel(format: string): string {
  const map: Record<string, string> = {
    flame: 'Flame',
    onepanel: '1Panel',
    baota: '宝塔',
  }
  return map[format] || format
}

/** 安装模式文案：wasm 为技术名词不翻译，未知模式原样透出 */
export function modeLabel(mode: string, t: (key: string) => string): string {
  if (mode === 'wasm') return 'WASM'
  if (mode === 'native') return t('appStore.modeNative')
  if (mode === 'container') return t('appStore.modeContainer')
  return mode
}

/** 模式标签配色（与列表 Tag 语义一致） */
export function modeTagType(mode: string): 'info' | 'success' | 'warning' {
  if (mode === 'wasm') return 'warning'
  if (mode === 'native') return 'success'
  return 'info'
}

/** 原生（native）模式判定——仅该模式提供 start/stop/restart 生命周期操作 */
export function isNativeMode(app: { mode?: string | null } | null | undefined): boolean {
  return app?.mode === 'native'
}
