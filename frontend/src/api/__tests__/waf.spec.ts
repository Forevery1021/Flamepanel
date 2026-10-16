import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：WAF 基础防护 API 层（§6.2）——断言请求路径与方法正确
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  return { default: { get, post, put, delete: del } }
})

import api from '@/api/client'
import {
  listWafRules,
  createWafRule,
  updateWafRule,
  deleteWafRule,
  listWafLogs,
  clearWafLogs,
  getWafStatus,
  applyWafRules,
} from '@/api/waf'

describe('waf API（WAF 基础防护）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listWafRules 走 GET /waf/rules', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listWafRules()
    expect(api.get).toHaveBeenCalledWith('/waf/rules', { params: { website_id: 0 } })
  })

  it('createWafRule 走 POST /waf/rules', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'bl', rule_type: 'ip_blacklist', pattern: '1.2.3.4', action: 'block' }
    await createWafRule(req)
    expect(api.post).toHaveBeenCalledWith('/waf/rules', req)
  })

  it('updateWafRule 走 PUT /waf/rules/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateWafRule(7, { enabled: false })
    expect(api.put).toHaveBeenCalledWith('/waf/rules/7', { enabled: false })
  })

  it('deleteWafRule 走 DELETE /waf/rules/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: null })
    await deleteWafRule(7)
    expect(api.delete).toHaveBeenCalledWith('/waf/rules/7')
  })

  it('listWafLogs 走 GET /waf/logs（分页 + 站点过滤）', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { data: [], total: 0 } })
    await listWafLogs(2, 20, 5)
    expect(api.get).toHaveBeenCalledWith('/waf/logs', {
      params: { page: 2, page_size: 20, website_id: 5 },
    })
  })

  it('clearWafLogs 走 POST /waf/logs/clear', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: 3 })
    await clearWafLogs(0)
    expect(api.post).toHaveBeenCalledWith('/waf/logs/clear', { website_id: 0 })
  })

  it('getWafStatus 走 GET /waf/status', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { loaded_rules: 2 } })
    await getWafStatus()
    expect(api.get).toHaveBeenCalledWith('/waf/status')
  })

  it('applyWafRules 走 POST /waf/apply', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { ok: true, engines: [] } })
    const { data } = await applyWafRules()
    expect(api.post).toHaveBeenCalledWith('/waf/apply')
    expect(data.ok).toBe(true)
  })
})
