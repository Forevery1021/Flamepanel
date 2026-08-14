import { defineStore } from 'pinia'
import { computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS } from '@/utils/storage'

export interface TabItem {
  path: string
  title: string
  pinned: boolean
}

const HOME_PATH = '/dashboard'
const DEFAULT_TABS: TabItem[] = [{ path: HOME_PATH, title: '', pinned: true }]

export const useTabsStore = defineStore('tabs', () => {
  // P6：改用 @vueuse/core useStorage 统一持久化
  const tabs = useStorage<TabItem[]>(STORAGE_KEYS.tabs, DEFAULT_TABS, undefined, {
    writeDefaults: false,
  })

  const paths = computed(() => tabs.value.map((t) => t.path))
  /** keep-alive include 用组件名（路由 name + 'View'） */
  const cachedNames = computed(() => tabs.value.map((t) => toComponentName(t.path)))

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
  }

  function close(path: string) {
    const idx = tabs.value.findIndex((t) => t.path === path)
    if (idx === -1 || tabs.value[idx]?.pinned) return
    tabs.value.splice(idx, 1)
  }

  function closeOthers(path: string) {
    tabs.value = tabs.value.filter((t) => t.path === path || t.pinned)
  }

  function closeAll() {
    tabs.value = tabs.value.filter((t) => t.pinned)
  }

  return { tabs, paths, cachedNames, open, close, closeOthers, closeAll }
})
