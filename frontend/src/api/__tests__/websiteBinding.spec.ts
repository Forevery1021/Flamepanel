import { describe, it, expect, vi, beforeEach } from 'vitest'
import api from '@/api/client'

vi.mock('@/api/auth', () => ({
  refreshToken: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

const calls: Array<{ method?: string; url?: string; data?: unknown }> = []

api.defaults.adapter = async (config) => {
  calls.push({ method: config.method, url: config.url, data: config.data })
  return {
    data: {},
    status: 200,
    statusText: 'OK',
    headers: {},
    config,
  } as never
}

import {
  bindWebsiteRuntime,
  applyWebsiteRewrite,
  getWebsiteRewrite,
  clearWebsiteRewrite,
  SITE_TYPES,
  REWRITE_PRESETS,
} from '@/api/websites'

describe('网站绑定 API（M-B Stage A-6 · Doc/20 §5.5）', () => {
  beforeEach(() => {
    calls.length = 0
  })

  it('bindWebsiteRuntime 走 POST /websites/:id/runtime 且携带 site_type', async () => {
    await bindWebsiteRuntime(7, { site_type: 'runtime_php', runtime_id: 3, project_id: null })
    const c = calls.find((x) => (x.url ?? '').endsWith('/websites/7/runtime'))
    expect(c).toBeTruthy()
    expect(c?.method).toBe('post')
    expect(JSON.parse(String(c?.data))).toEqual({
      site_type: 'runtime_php',
      runtime_id: 3,
      project_id: null,
    })
  })

  it('applyWebsiteRewrite 走 POST /websites/:id/rewrite 且携带 preset', async () => {
    await applyWebsiteRewrite(7, 'wordpress')
    const c = calls.find((x) => (x.url ?? '').endsWith('/websites/7/rewrite'))
    expect(c).toBeTruthy()
    expect(JSON.parse(String(c?.data))).toEqual({ preset: 'wordpress' })
  })

  it('getWebsiteRewrite 走 GET /websites/:id/rewrite', async () => {
    await getWebsiteRewrite(7)
    const c = calls.find((x) => (x.url ?? '').endsWith('/websites/7/rewrite'))
    expect(c).toBeTruthy()
    expect(c?.method).toBe('get')
  })

  it('clearWebsiteRewrite 走 DELETE /websites/:id/rewrite', async () => {
    await clearWebsiteRewrite(7)
    const c = calls.find((x) => (x.url ?? '').endsWith('/websites/7/rewrite'))
    expect(c).toBeTruthy()
    expect(c?.method).toBe('delete')
  })

  it('常量白名单与后端对齐', () => {
    expect(SITE_TYPES).toEqual(['runtime_php', 'reverse_proxy', 'static', 'container_app'])
    expect(REWRITE_PRESETS).toContain('wordpress')
    expect(REWRITE_PRESETS).toHaveLength(6)
  })
})
