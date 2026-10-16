import type { Serializer } from '@vueuse/core'

/**
 * P6 · VueUse 存储统一。
 *
 * 统一提供 localStorage 的 key 常量与序列化器，配合 @vueuse/core 的 `useStorage`
 * 使用，替换各 store/工具中散落的 `localStorage.getItem/setItem`。
 *
 * - 对象/数组/布尔等结构值：直接使用 `useStorage` 默认 JSON 序列化（兼容既有 JSON 存储格式）。
 * - 单一字符串值（如主题 mode、语言、token）：使用 `rawStringSerializer`，保持既有
 *   未加引号的原始字符串格式，避免与旧数据/直接读取点产生格式漂移。
 */

/** localStorage key 常量（集中管理，避免魔法字符串） */
export const STORAGE_KEYS = {
  /** 主题明暗模式 */
  mode: 'flamepanel.mode',
  /** 主题预设 */
  preset: 'flamepanel.theme',
  /** 自定义主题 */
  custom: 'flamepanel.custom',
  /** 玻璃总开关 */
  glass: 'flamepanel.glass',
  /** 表面材质（solid / glass / acrylic）；缺省时由 glass 开关迁移 */
  material: 'flamepanel.material',
  /** 界面外观偏好 */
  appearance: 'flamepanel.appearance',
  /** 侧边栏展开分组 */
  openGroups: 'flamepanel.sidebar.openGroups',
  /** 新手引导完成标记（按用户名后缀） */
  onboarding: 'flamepanel.onboarding',
  /** Setup 向导草稿（sessionStorage，刷新不丢） */
  setupDraft: 'flamepanel.setup.draft',
  /** S-17 仪表盘安全体检横幅「本次忽略」（sessionStorage，按报告生成时间记忆） */
  inspectionBannerDismissed: 'flamepanel.inspection.bannerDismissed',
  /** S-15 ⌘K 最近访问的菜单路由名（按用户名后缀） */
  paletteRecent: 'flamepanel.palette.recent',
  /** 侧边栏折叠状态 */
  collapse: 'flamepanel.sidebar.collapsed',
  /** 语言 */
  lang: 'flame-lang',
  /** 访问令牌 */
  token: 'token',
  /** 刷新令牌 */
  refreshToken: 'refresh_token',
  /** 用户名 */
  username: 'username',
  /** 角色 */
  role: 'role',
} as const

/** 不改变存储格式的原始字符串序列化器（写入不加引号，读取原样返回）。
 *  泛型化以适配字符串字面量联合类型（如 ThemeMode / AppLocale）。 */
export function rawStringSerializer<T extends string>(): Serializer<T> {
  return {
    read: (raw) => raw as T,
    write: (value) => value,
  }
}

/** '1'/'0' 布尔序列化器（兼容既有折叠状态存储格式） */
export const rawBooleanSerializer: Serializer<boolean> = {
  read: (raw) => raw === '1',
  write: (value) => (value ? '1' : '0'),
}

/**
 * 直写 localStorage 并广播变更（同文档同步）。
 *
 * `@vueuse/core` 的 `useStorage` 只监听浏览器原生 `storage` 事件，而该事件**仅在其它
 * 文档**触发——同一文档内的直写不会通知任何订阅者，于是出现「localStorage 已是新值、
 * 订阅它的 store 仍是旧值」的分叉。
 *
 * 常规读写请优先使用 `useStorage`（store 侧，见 `stores/auth.ts`）；本函数只服务于
 * **无法访问 store 的底层模块**——典型是 `api/client.ts` 的 401 刷新回写：它被 store
 * 反向依赖（store → api/auth → client），不能 import store，否则成环。
 *
 * `value` 传 `null` 表示移除该键。
 */
export function writeStorage(key: string, value: string | null): void {
  const oldValue = localStorage.getItem(key)
  if (value === null) {
    localStorage.removeItem(key)
  } else {
    localStorage.setItem(key, value)
  }
  // 同文档广播：让 useStorage 订阅者（auth store）与存储保持一致（值未变则不广播）
  if (oldValue === value) return
  window.dispatchEvent(
    new StorageEvent('storage', { key, oldValue, newValue: value, storageArea: localStorage }),
  )
}
