const MODE_KEY = 'flamepanel.mode'
const LEGACY_KEY = 'flame-theme'

export function isDark(): boolean {
  return document.documentElement.classList.contains('dark')
}

export function applyTheme(theme: 'light' | 'dark'): void {
  document.documentElement.classList.toggle('dark', theme === 'dark')
  localStorage.setItem(MODE_KEY, theme)
}

/** 挂载前同步明暗模式，避免首屏闪烁（兼容旧 key） */
export function applyStoredTheme(): void {
  const stored = localStorage.getItem(MODE_KEY) ?? localStorage.getItem(LEGACY_KEY)
  if (stored === 'light' || stored === 'dark') {
    document.documentElement.classList.toggle('dark', stored === 'dark')
    return
  }
  const prefersDark =
    window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
  document.documentElement.classList.toggle('dark', prefersDark)
}
