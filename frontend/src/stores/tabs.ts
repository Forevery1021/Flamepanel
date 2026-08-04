import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface TabItem {
  path: string
  title: string
  pinned: boolean
}

const TABS_KEY = 'flamepanel.tabs'
const HOME_PATH = '/dashboard'

export const useTabsStore = defineStore('tabs', () => {
  const tabs = ref<TabItem[]>(loadTabs())

  const paths = computed(() => tabs.value.map((t) => t.path))
  /** keep-alive include 用组件名（路由 name + 'View'） */
  const cachedNames = computed(() => tabs.value.map((t) => toComponentName(t.path)))

  function loadTabs(): TabItem[] {
    try {
      const raw = localStorage.getItem(TABS_KEY)
      const parsed = raw ? JSON.parse(raw) : []
      if (Array.isArray(parsed)) return parsed
    } catch {
      // ignore
    }
    return [{ path: HOME_PATH, title: '', pinned: true }]
  }

  function persist() {
    localStorage.setItem(TABS_KEY, JSON.stringify(tabs.value))
  }

  function toComponentName(path: string): string {
    const name = path.split('/').filter(Boolean).join('-')
    if (!name) return 'DashboardView'
    const upper = name
      .split('-')
      .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
      .join('')
    return `${upper}View`
  }

  function open(path: string, title: string) {
    const existing = tabs.value.find((t) => t.path === path)
    if (existing) {
      existing.title = title
      return
    }
    tabs.value.push({ path, title, pinned: path === HOME_PATH })
    if (tabs.value.length > 12) {
      const removable = tabs.value.find((t) => !t.pinned)
      if (removable) close(removable.path)
    }
    persist()
  }

  function close(path: string) {
    const idx = tabs.value.findIndex((t) => t.path === path)
    if (idx === -1 || tabs.value[idx]?.pinned) return
    tabs.value.splice(idx, 1)
    persist()
  }

  function closeOthers(path: string) {
    tabs.value = tabs.value.filter((t) => t.path === path || t.pinned)
    persist()
  }

  function closeAll() {
    tabs.value = tabs.value.filter((t) => t.pinned)
    persist()
  }

  return { tabs, paths, cachedNames, open, close, closeOthers, closeAll }
})
