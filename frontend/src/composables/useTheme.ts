import { ref, watch } from 'vue'
import api from '@/api/client'
import type { PanelSettings } from '@/types'

const currentTheme = ref<string>(localStorage.getItem('theme') || 'light')

export function useTheme() {
  const applyTheme = (theme: string) => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('theme', theme)
    currentTheme.value = theme
  }

  const toggleTheme = () => {
    const next = currentTheme.value === 'dark' ? 'light' : 'dark'
    applyTheme(next)
    api.put('/settings', { theme: next }).catch(() => {})
  }

  const initTheme = async () => {
    // Apply saved theme immediately to prevent flash
    applyTheme(currentTheme.value)
    // Then sync with server
    try {
      const res = await api.get<PanelSettings>('/settings')
      const serverTheme = res.data.theme || 'light'
      if (serverTheme !== currentTheme.value) {
        applyTheme(serverTheme)
      }
    } catch {
      // Keep localStorage theme if server is unavailable
    }
  }

  return { currentTheme, applyTheme, toggleTheme, initTheme }
}
