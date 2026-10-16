import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：远程源 API 层（M-A Issue 1.4）—— 断言请求路径与方法正确
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  const longApi = { post: vi.fn() }
  return { default: { get, post, put, delete: del }, longApi }
})

import api, { longApi } from '@/api/client'
import {
  listSources,
  createSource,
  updateSource,
  deleteSource,
  syncSource,
  listSourcePackages,
  getStoreSecurity,
  updateStoreSecurity,
} from '@/api/appStore'

describe('appStore 远程源 API（M-A Issue 1.4）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listSources 走 GET /app-store/sources', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { total: 0, sources: [] } })
    await listSources()
    expect(api.get).toHaveBeenCalledWith('/app-store/sources')
  })

  it('createSource 走 longApi POST（长操作：添加即同步索引）', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 's', source_type: 'git', url: 'https://x' }
    await createSource(req)
    expect(longApi.post).toHaveBeenCalledWith('/app-store/sources', req)
  })

  it('updateSource 走 PUT /app-store/sources/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateSource(3, { enabled: false })
    expect(api.put).toHaveBeenCalledWith('/app-store/sources/3', { enabled: false })
  })

  it('deleteSource 走 DELETE /app-store/sources/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { success: true } })
    await deleteSource(5)
    expect(api.delete).toHaveBeenCalledWith('/app-store/sources/5')
  })

  it('syncSource 走 longApi POST /app-store/sources/:id/sync（长操作）', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { added: 1, skipped: 0, packages: [] } })
    await syncSource(7)
    expect(longApi.post).toHaveBeenCalledWith('/app-store/sources/7/sync')
  })

  it('listSourcePackages 走 GET /app-store/sources/:id/packages', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { total: 0, packages: [] } })
    await listSourcePackages(9)
    expect(api.get).toHaveBeenCalledWith('/app-store/sources/9/packages')
  })

  // ── 商店安全配置（Doc/20 §4.2 收口） ──

  it('createSource 未签名源携带 confirm_unsigned（二次确认透传后端门禁）', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = {
      name: 'unsigned',
      source_type: 'git',
      url: 'https://u.example.com/r',
      confirm_unsigned: true,
    }
    await createSource(req)
    expect(longApi.post).toHaveBeenCalledWith('/app-store/sources', req)
  })

  it('updateSource 清空公钥携带 confirm_unsigned（信任锚卸载须确认）', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateSource(3, { public_key: null, confirm_unsigned: true })
    expect(api.put).toHaveBeenCalledWith('/app-store/sources/3', {
      public_key: null,
      confirm_unsigned: true,
    })
  })

  it('getStoreSecurity 走 GET /app-store/security', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await getStoreSecurity()
    expect(api.get).toHaveBeenCalledWith('/app-store/security')
  })

  it('updateStoreSecurity 走 PUT /app-store/security（策略 + 白名单全量替换）', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = {
      signature_policy: 'required',
      registry_whitelist: ['registry.example.com'],
    }
    await updateStoreSecurity(req)
    expect(api.put).toHaveBeenCalledWith('/app-store/security', req)
  })
})
