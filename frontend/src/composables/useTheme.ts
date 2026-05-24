import { ref, watch } from 'vue'
import api from '@/api/client'
import type { PanelSettings } from '@/types'

export type ThemeColor = 'blue' | 'green' | 'purple' | 'orange' | 'red' | 'cyan'
export type ThemeMode = 'light' | 'dark'

const STORAGE_KEY = 'flamepanel_theme'
const COLOR_KEY = 'flamepanel_theme_color'
const BG_KEY = 'flamepanel_bg_image'
const BG_OPACITY_KEY = 'flamepanel_bg_opacity'

const currentTheme = ref<ThemeMode>(
  (localStorage.getItem(STORAGE_KEY) as ThemeMode) || 'light'
)
const currentColor = ref<ThemeColor>(
  (localStorage.getItem(COLOR_KEY) as ThemeColor) || 'blue'
)
const backgroundImage = ref<string>(
  localStorage.getItem(BG_KEY) || ''
)
const backgroundOpacity = ref<number>(
  parseFloat(localStorage.getItem(BG_OPACITY_KEY) || '0.4')
)

export function useTheme() {
  const applyTheme = (theme: ThemeMode) => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem(STORAGE_KEY, theme)
    currentTheme.value = theme
  }

  const applyColor = (color: ThemeColor) => {
    document.documentElement.setAttribute('data-theme-color', color)
    localStorage.setItem(COLOR_KEY, color)
    currentColor.value = color
    // Force Element Plus to pick up new primary color
    const root = document.documentElement
    root.style.setProperty('--el-color-primary', getComputedStyle(root).getPropertyValue('--el-color-primary'))
  }

  const applyBackground = (url: string, opacity?: number) => {
    backgroundImage.value = url
    if (opacity !== undefined) backgroundOpacity.value = opacity
    localStorage.setItem(BG_KEY, url)
    if (opacity !== undefined) localStorage.setItem(BG_OPACITY_KEY, String(opacity))

    if (url) {
      document.documentElement.setAttribute('data-bg-image', 'true')
      document.documentElement.style.setProperty('--bg-image-url', `url(${url})`)
      document.documentElement.style.setProperty(
        '--bg-image-opacity',
        String(opacity ?? backgroundOpacity.value)
      )
    } else {
      document.documentElement.removeAttribute('data-bg-image')
      document.documentElement.style.removeProperty('--bg-image-url')
      document.documentElement.style.removeProperty('--bg-image-opacity')
    }
  }

  const removeBackground = () => {
    applyBackground('')
  }

  const toggleTheme = () => {
    const next = currentTheme.value === 'dark' ? 'light' : 'dark'
    applyTheme(next)
    api.put('/settings', { theme: next }).catch(() => {})
  }

  const syncSettings = async () => {
    await api.put('/settings', {
      theme: currentTheme.value,
      language: localStorage.getItem('flamepanel_language') || 'zh-CN',
      theme_color: currentColor.value,
      background_image: backgroundImage.value,
      background_opacity: backgroundOpacity.value,
    }).catch(() => {})
  }

  const initTheme = async () => {
    // Apply local settings immediately to prevent flash
    applyTheme(currentTheme.value)
    applyColor(currentColor.value)
    if (backgroundImage.value) {
      applyBackground(backgroundImage.value, backgroundOpacity.value)
    }

    // Sync with server
    try {
      const res = await api.get<PanelSettings>('/settings')
      const s = res.data
      if (s.theme && s.theme !== currentTheme.value) applyTheme(s.theme as ThemeMode)
      if (s.theme_color && s.theme_color !== currentColor.value) applyColor(s.theme_color as ThemeColor)
      if (s.background_image && s.background_image !== backgroundImage.value) {
        applyBackground(s.background_image, s.background_opacity ?? 0.4)
      } else if (!s.background_image && backgroundImage.value) {
        removeBackground()
      }
    } catch {
      // Keep localStorage settings if server unavailable
    }
  }

  return {
    currentTheme,
    currentColor,
    backgroundImage,
    backgroundOpacity,
    applyTheme,
    applyColor,
    applyBackground,
    removeBackground,
    toggleTheme,
    initTheme,
    syncSettings,
  }
}
