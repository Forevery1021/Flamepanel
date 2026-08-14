import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * 界面外观偏好（多页签/手风琴/隐藏菜单/折叠状态/分组展开）。
 * 本地持久化 + 后端设置双向同步（服务端为权威，登录时 syncFromServer 覆盖）。
 */

export interface Appearance {
  menuTabs: boolean
  menuAccordion: boolean
  hideMenu: string[]
  menuCollapsed: boolean
}

const DEFAULT_APPEARANCE: Appearance = {
  menuTabs: true,
  menuAccordion: false,
  hideMenu: [],
  menuCollapsed: false,
}

export const useAppearanceStore = defineStore('appearance', () => {
  // P6：改用 @vueuse/core useStorage 统一持久化
  const state = useStorage<Appearance>(STORAGE_KEYS.appearance, DEFAULT_APPEARANCE, undefined, {
    mergeDefaults: true,
  })
  /** 展开的侧边栏分组（main 始终可见，不在此列） */
  const openGroups = useStorage<string[]>(STORAGE_KEYS.openGroups, [])
  /** 用户是否手动自定义过分组展开（避免覆盖用户选择）。
   *  仅作为运行期标记：只要 openGroups 已持久化过（键存在）即为 true，toggleGroup 亦会置位。 */
  const groupsCustomized = ref(localStorage.getItem(STORAGE_KEYS.openGroups) !== null)

  /** 按角色返回默认展开的次要分组：
   *  admin：默认展开高频运维分组（web/storage/ops），折叠低频（apps/system）
   *  operator：默认展开运维相关（ops/storage），折叠其余
   *  viewer：默认折叠全部次要分组，仅保留 main */
  function defaultOpenGroupsForRole(role: string): string[] {
    if (role === 'admin') return ['web', 'storage', 'ops']
    if (role === 'operator') return ['ops', 'storage']
    return []
  }

  function update(patch: Partial<Appearance>) {
    state.value = { ...state.value, ...patch }
  }

  /** 登录后按角色应用默认分组展开（仅当用户尚未手动自定义分组时） */
  function applyRoleDefaults(role: string) {
    if (groupsCustomized.value) return
    const defaults = defaultOpenGroupsForRole(role)
    openGroups.value = defaults
  }

  /** 展开/收起分组；手风琴模式下同时只展开一个 */
  function toggleGroup(key: string) {
    groupsCustomized.value = true
    if (openGroups.value.includes(key)) {
      openGroups.value = openGroups.value.filter((g) => g !== key)
    } else {
      openGroups.value = state.value.menuAccordion
        ? [key]
        : [...openGroups.value, key]
    }
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

  return {
    state,
    openGroups,
    groupsCustomized,
    update,
    toggleGroup,
    isGroupOpen,
    applyRoleDefaults,
    syncFromServer,
  }
})
