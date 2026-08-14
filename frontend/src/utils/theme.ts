import { STORAGE_KEYS } from './storage'

const LEGACY_KEY = 'flame-theme'

export function isDark(): boolean {
  return document.documentElement.classList.contains('dark')
}

export function applyTheme(theme: 'light' | 'dark'): void {
  document.documentElement.classList.toggle('dark', theme === 'dark')
  localStorage.setItem(STORAGE_KEYS.mode, theme)
}

/** 挂载前同步玻璃开关，避免闪烁/误渲染 */
export function applyStoredGlass(): void {
  document.documentElement.classList.toggle(
    'glass-disabled',
    localStorage.getItem(STORAGE_KEYS.glass) === 'false',
  )
}

/** 挂载前同步明暗模式，避免首屏闪烁（兼容旧 key） */
export function applyStoredTheme(): void {
  const stored = localStorage.getItem(STORAGE_KEYS.mode) ?? localStorage.getItem(LEGACY_KEY)
  if (stored === 'light' || stored === 'dark') {
    document.documentElement.classList.toggle('dark', stored === 'dark')
  } else {
    const prefersDark =
      window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
    document.documentElement.classList.toggle('dark', prefersDark)
  }
  applyStoredGlass()
}
