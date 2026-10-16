/**
 * ⌘K「最近访问」（Doc/30 S-15）。
 *
 * - 只记录**菜单路由**的路由名（`meta.group` 非空），不记录 query / hash——命令面板的导航命令
 *   同样只到页面级；
 * - 按用户名隔离（与新手引导标记同一约定），同一浏览器的 admin / operator 互不串；
 * - 最新在前、去重、上限 {@link RECENT_ROUTE_LIMIT} 条；存储损坏时回落空表，绝不抛错。
 *
 * 权限过滤不在这里做：面板展示时与导航命令求交集（导航命令已按角色过滤），
 * 因此角色降级后旧记录会自然隐藏而不是变成死链。
 */
import { STORAGE_KEYS } from '@/utils/storage'

export const RECENT_ROUTE_LIMIT = 5

function storageKey(username: string): string {
  return `${STORAGE_KEYS.paletteRecent}.${username || 'anonymous'}`
}

/** 纯函数：把 `name` 置顶（去重 + 截断） */
export function pushRecent(
  list: readonly string[],
  name: string,
  limit = RECENT_ROUTE_LIMIT,
): string[] {
  if (!name) return list.slice(0, limit)
  return [name, ...list.filter((n) => n !== name)].slice(0, limit)
}

export function readRecentRoutes(username: string): string[] {
  try {
    const raw = localStorage.getItem(storageKey(username))
    if (!raw) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter((n): n is string => typeof n === 'string' && n.length > 0)
      .slice(0, RECENT_ROUTE_LIMIT)
  } catch {
    return []
  }
}

export function recordRecentRoute(username: string, name: string): void {
  try {
    localStorage.setItem(
      storageKey(username),
      JSON.stringify(pushRecent(readRecentRoutes(username), name)),
    )
  } catch {
    // 隐私模式 / 配额满：最近访问只是便利功能，静默失败
  }
}
