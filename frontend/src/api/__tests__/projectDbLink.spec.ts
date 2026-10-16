import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：项目数据库联链 API 层（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
// 断言请求路径与方法正确；连接串由后端在启动/部署时生成，前端不触碰凭据
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  return { default: { get, post, put, delete: del } }
})

import api from '@/api/client'
import {
  listProjectLinks,
  listLinkableDatabases,
  linkProjectDatabase,
  unlinkProjectDatabase,
} from '@/api/projects'

describe('projectDbLink API（服务联链 · Dokku link 模式）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listProjectLinks 走 GET /projects/:id/links', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listProjectLinks(7)
    expect(api.get).toHaveBeenCalledWith('/projects/7/links')
  })

  it('listLinkableDatabases 走 GET /projects/:id/links/available', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listLinkableDatabases(7)
    expect(api.get).toHaveBeenCalledWith('/projects/7/links/available')
  })

  it('linkProjectDatabase 走 POST /projects/:id/links', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { id: 1 } })
    const req = { db_instance_id: 2, env_name: null, db_name: null }
    await linkProjectDatabase(7, req)
    expect(api.post).toHaveBeenCalledWith('/projects/7/links', req)
  })

  it('unlinkProjectDatabase 走 DELETE /projects/:id/links/:linkId', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { ok: true } })
    await unlinkProjectDatabase(7, 1)
    expect(api.delete).toHaveBeenCalledWith('/projects/7/links/1')
  })

  it('queryKeys.projects.links 稳定（列表缓存键不随参数漂移）', async () => {
    const { queryKeys } = await import('@/api/queryKeys')
    expect(queryKeys.projects.links(7)).toEqual(['projects', 'links', 7])
  })
})
