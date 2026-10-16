import { describe, it, expect, beforeEach } from 'vitest'
import {
  RECENT_ROUTE_LIMIT,
  pushRecent,
  readRecentRoutes,
  recordRecentRoute,
} from '@/utils/recentRoutes'

/** S-15 ⌘K 最近访问：置顶去重、上限 5、按用户隔离、存储损坏回落空表 */
describe('recentRoutes', () => {
  beforeEach(() => localStorage.clear())

  it('pushRecent 置顶、去重并截断到上限', () => {
    expect(pushRecent([], 'Dashboard')).toEqual(['Dashboard'])
    expect(pushRecent(['A', 'B', 'C'], 'B')).toEqual(['B', 'A', 'C'])
    const full = ['A', 'B', 'C', 'D', 'E']
    expect(pushRecent(full, 'F')).toEqual(['F', 'A', 'B', 'C', 'D'])
    expect(pushRecent(full, 'F')).toHaveLength(RECENT_ROUTE_LIMIT)
    // 空名不入表
    expect(pushRecent(['A'], '')).toEqual(['A'])
  })

  it('按用户名隔离记录，最新在前', () => {
    recordRecentRoute('admin', 'Dashboard')
    recordRecentRoute('admin', 'Firewall')
    recordRecentRoute('admin', 'Dashboard')
    recordRecentRoute('ops', 'Memos')
    expect(readRecentRoutes('admin')).toEqual(['Dashboard', 'Firewall'])
    expect(readRecentRoutes('ops')).toEqual(['Memos'])
    expect(readRecentRoutes('nobody')).toEqual([])
  })

  it('存储内容损坏或类型不符时回落空表 / 过滤非法项', () => {
    localStorage.setItem('flamepanel.palette.recent.admin', '{not json')
    expect(readRecentRoutes('admin')).toEqual([])
    localStorage.setItem('flamepanel.palette.recent.admin', JSON.stringify({ a: 1 }))
    expect(readRecentRoutes('admin')).toEqual([])
    localStorage.setItem('flamepanel.palette.recent.admin', JSON.stringify(['A', 3, '', 'B']))
    expect(readRecentRoutes('admin')).toEqual(['A', 'B'])
  })
})
