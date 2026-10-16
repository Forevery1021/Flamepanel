import { STORAGE_KEYS } from './storage'

const LEGACY_KEY = 'flame-theme'

export function isDark(): boolean {
  return document.documentElement.classList.contains('dark')
}

export function applyTheme(theme: 'light' | 'dark'): void {
  document.documentElement.classList.toggle('dark', theme === 'dark')
  localStorage.setItem(STORAGE_KEYS.mode, theme)
}

/** 挂载前同步材质档位（兼容旧版布尔 glass 开关），与 index.html 首屏脚本同一规则 */
export function applyStoredGlass(): void {
  let material = localStorage.getItem(STORAGE_KEYS.material)
  if (material !== 'solid' && material !== 'glass' && material !== 'acrylic') {
    material = localStorage.getItem(STORAGE_KEYS.glass) === 'false' ? 'solid' : 'glass'
  }
  const root = document.documentElement
  root.dataset.material = material
  root.classList.toggle('glass-disabled', material === 'solid')
}

/**
 * 挂载前同步明暗模式，避免首屏闪烁（兼容旧 key）。
 * 规则与 stores/theme.ts 一致：未设置 → 深色；`auto` → 跟随 prefers-color-scheme。
 */
export function applyStoredTheme(): void {
  const stored = localStorage.getItem(STORAGE_KEYS.mode) ?? localStorage.getItem(LEGACY_KEY)
  let dark = true
  if (stored === 'light') dark = false
  else if (stored === 'auto') {
    dark = !!window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
  }
  document.documentElement.classList.toggle('dark', dark)
  applyStoredGlass()
}
