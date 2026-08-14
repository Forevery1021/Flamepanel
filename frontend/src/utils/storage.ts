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
  /** 界面外观偏好 */
  appearance: 'flamepanel.appearance',
  /** 侧边栏展开分组 */
  openGroups: 'flamepanel.sidebar.openGroups',
  /** 多页签列表 */
  tabs: 'flamepanel.tabs',
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
