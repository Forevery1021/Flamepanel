import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

/**
 * 界面外观偏好（多页签/手风琴/隐藏菜单/折叠状态/分组展开）。
 * 本地持久化 + 后端设置双向同步（服务端为权威，登录时 syncFromServer 覆盖）。
 */
const KEY = 'flamepanel.appearance'
const OPEN_GROUPS_KEY = 'flamepanel.sidebar.openGroups'

export interface Appearance {
  menuTabs: boolean
  menuAccordion: boolean
  hideMenu: string[]
  menuCollapsed: boolean
}

function load(): Appearance {
  try {
    const raw = localStorage.getItem(KEY)
    const p = raw ? JSON.parse(raw) : {}
    return {
      menuTabs: p.menuTabs ?? true,
      menuAccordion: p.menuAccordion ?? false,
      hideMenu: Array.isArray(p.hideMenu) ? p.hideMenu : [],
      menuCollapsed: p.menuCollapsed ?? false,
    }
  } catch {
    return { menuTabs: true, menuAccordion: false, hideMenu: [], menuCollapsed: false }
  }
}

function loadOpenGroups(): string[] {
  try {
    const raw = localStorage.getItem(OPEN_GROUPS_KEY)
    const arr = raw ? JSON.parse(raw) : []
    return Array.isArray(arr) ? arr : []
  } catch {
    return []
  }
}

export const useAppearanceStore = defineStore('appearance', () => {
  const state = ref<Appearance>(load())
  /** 展开的侧边栏分组（main 始终可见，不在此列） */
  const openGroups = ref<string[]>(loadOpenGroups())

  function persist() {
    localStorage.setItem(KEY, JSON.stringify(state.value))
  }

  function update(patch: Partial<Appearance>) {
    state.value = { ...state.value, ...patch }
    persist()
  }

  /** 展开/收起分组；手风琴模式下同时只展开一个 */
  function toggleGroup(key: string) {
    if (openGroups.value.includes(key)) {
      openGroups.value = openGroups.value.filter((g) => g !== key)
    } else {
      openGroups.value = state.value.menuAccordion
        ? [key]
        : [...openGroups.value, key]
    }
    localStorage.setItem(OPEN_GROUPS_KEY, JSON.stringify(openGroups.value))
  }

  function isGroupOpen(key: string) {
    return openGroups.value.includes(key)
  }

  /** 从后端设置同步（登录后调用；后端空值不覆盖本地） */
  function syncFromServer(map: Record<string, string>) {
    const patch: Partial<Appearance> = {}
    if (map['open_menu_tabs']) patch.menuTabs = map['open_menu_tabs'] === 'true'
    if (map['menu_accordion']) patch.menuAccordion = map['menu_accordion'] === 'true'
    if (map['menu_collapsed']) patch.menuCollapsed = map['menu_collapsed'] === 'true'
    if (map['hide_menu']) {
      try {
        const arr = JSON.parse(map['hide_menu'])
        if (Array.isArray(arr)) patch.hideMenu = arr
      } catch {
        // ignore
      }
    }
    if (Object.keys(patch).length) update(patch)
  }

  watch(state, persist, { deep: true })

  return { state, openGroups, update, toggleGroup, isGroupOpen, syncFromServer }
})
