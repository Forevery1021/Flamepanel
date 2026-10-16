import { describe, it, expect } from 'vitest'
import {
  hasPermission,
  hasRoutePermission,
  isRouteVisibleInMenu,
} from '@/directives/permission'
import { menuRoutes } from '@/router'

/**
 * T0-1 路由级 RBAC 守卫（Doc/23 §3.1 / 问题 P0-2）回归保护。
 *
 * 修复前的缺口：`beforeEach` 只查登录态，`isMenuHiddenForRole` 又只作用于侧栏，
 * 于是 viewer 直接输入 `/users` URL 即可进入页面（API 虽 403，但页面框架照常渲染），
 * ⌘K 命令面板也照旧列出无权限入口。
 *
 * 验收点（与任务单逐条对应）：
 * 1. viewer 直接访问 `/users` 被判「无权限」（守卫生效）；
 * 2. admin 不受影响；
 * 3. 三条消费方（守卫 `hasRoutePermission` / 菜单 `isRouteVisibleInMenu`）判据同源，
 *    不存在「菜单可见但守卫拒绝」的死路。
 *
 * 注：这些用例锁定的是**页面框架观感**，安全边界仍在服务端 RBAC（Doc/07 §3）。
 */

/** 构造 meta（`hasRoutePermission` 只消费 meta；菜单判定才需要整条路由） */
const meta = (perm?: string) => (perm ? { perm } : {})

describe('hasRoutePermission（守卫判据，T0-1）', () => {
  it('【关键回归】viewer 访问 /users 被拒（历史缺口：URL 直达可进）', () => {
    expect(hasRoutePermission('viewer', meta('user:read'))).toBe(false)
  })

  it('admin 不受影响：全部页面放行', () => {
    for (const r of menuRoutes) {
      expect(hasRoutePermission('admin', r.meta)).toBe(true)
    }
  })

  it('viewer 仅 read 能力：读页面放行、写能力面拒绝', () => {
    expect(hasRoutePermission('viewer', meta('node:read'))).toBe(true)
    expect(hasRoutePermission('viewer', meta('website:read'))).toBe(true)
    // terminal:execute 是 activate 能力，viewer 无
    expect(hasRoutePermission('viewer', meta('terminal:execute'))).toBe(false)
  })

  it('operator 可进读页面；用户管理（管理类）被拒', () => {
    expect(hasRoutePermission('operator', meta('node:read'))).toBe(true)
    expect(hasRoutePermission('operator', meta('user:read'))).toBe(false)
  })

  it('无权限点的页面放行（/dashboard 是守卫的兜底重定向目标，不得被拒）', () => {
    expect(hasRoutePermission('viewer', meta())).toBe(true)
    expect(hasRoutePermission('viewer', meta())).toBe(true)
    expect(hasRoutePermission('viewer', meta())).toBe(true)
  })

  it('角色为空 → 放行（服务端角色尚未同步的过渡态，避免整站误锁）', () => {
    expect(hasRoutePermission('', meta('user:read'))).toBe(true)
  })

  it('未知/自定义角色 → 放行（前端无法表达其权限集，误锁代价更高）', () => {
    expect(hasRoutePermission('custom-role', meta('user:read'))).toBe(true)
  })
})

describe('菜单与守卫同源（无死路）', () => {
  it('【关键回归】菜单可见 ⇒ 守卫放行（对所有内置角色遍历路由表）', () => {
    for (const role of ['admin', 'operator', 'viewer']) {
      for (const r of menuRoutes) {
        if (!isRouteVisibleInMenu(role, r)) continue
        expect(
          hasRoutePermission(role, r.meta),
          `${role} 可看到 ${r.path} 菜单，守卫却拒绝 → 死路`,
        ).toBe(true)
      }
    }
  })

  it('viewer 的侧栏不再列出用户管理（既有规则保持）', () => {
    const users = menuRoutes.find((r) => r.path === '/users')!
    expect(isRouteVisibleInMenu('viewer', users)).toBe(false)
    expect(isRouteVisibleInMenu('operator', users)).toBe(false)
    expect(isRouteVisibleInMenu('admin', users)).toBe(true)
  })

  it('viewer 的终端入口与守卫同步隐藏（否则点入即被弹回）', () => {
    const terminal = menuRoutes.find((r) => r.path === '/terminal')!
    expect(isRouteVisibleInMenu('viewer', terminal)).toBe(false)
    expect(hasRoutePermission('viewer', terminal.meta)).toBe(false)
    // admin/operator 两端一致可见且可进
    for (const role of ['admin', 'operator']) {
      expect(isRouteVisibleInMenu(role, terminal)).toBe(true)
      expect(hasRoutePermission(role, terminal.meta)).toBe(true)
    }
  })
})

describe('meta.perm 覆盖度（与后端权限点字面对齐）', () => {
  it('除放行白名单外，所有菜单路由都声明了 meta.perm', () => {
    // 白名单 = 无资源级权限点的落地页/自助页，见 RouteMeta.perm 注释
    const NO_PERM = ['/dashboard', '/settings', '/operation-logs', '/api-docs']
    const missing = menuRoutes
      .filter((r) => !NO_PERM.includes(r.path))
      .filter((r) => !r.meta?.perm)
      .map((r) => r.path)
    expect(missing, `以下页面缺少 meta.perm：${missing.join(', ')}`).toEqual([])
  })

  it('meta.perm 均为合法 resource:action 且能被前端矩阵消费', () => {
    for (const r of menuRoutes) {
      const perm = r.meta?.perm
      if (!perm) continue
      expect(perm, `${r.path} 的 perm 格式不合法`).toMatch(/^[a-z_]+:[a-z_]+$/)
      // admin 必须能消费全部权限点（防止写出矩阵外的资源名）
      expect(hasPermission('admin', perm)).toBe(true)
    }
  })
})
