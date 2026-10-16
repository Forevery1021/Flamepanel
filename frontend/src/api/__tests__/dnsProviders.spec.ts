import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：DNS 厂商账号 API 层（§6.1 Stage 4）——断言请求路径与方法正确
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  return { default: { get, post, put, delete: del } }
})

import api from '@/api/client'
import {
  listDnsProviders,
  createDnsProvider,
  updateDnsProvider,
  deleteDnsProvider,
  testDnsProvider,
} from '@/api/dnsProviders'

describe('dnsProviders API（DNS 厂商账号）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listDnsProviders 走 GET /dns-providers', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listDnsProviders()
    expect(api.get).toHaveBeenCalledWith('/dns-providers')
  })

  it('createDnsProvider 走 POST /dns-providers', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'cf', provider_type: 'cloudflare', credentials_json: '{"api_token":"t"}' }
    await createDnsProvider(req)
    expect(api.post).toHaveBeenCalledWith('/dns-providers', req)
  })

  it('updateDnsProvider 走 PUT /dns-providers/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateDnsProvider(3, { enabled: false })
    expect(api.put).toHaveBeenCalledWith('/dns-providers/3', { enabled: false })
  })

  it('deleteDnsProvider 走 DELETE /dns-providers/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await deleteDnsProvider(5)
    expect(api.delete).toHaveBeenCalledWith('/dns-providers/5')
  })

  it('testDnsProvider 走 POST /dns-providers/:id/test', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { last_test_status: 'ok' } })
    await testDnsProvider(7)
    expect(api.post).toHaveBeenCalledWith('/dns-providers/7/test')
  })
})

// Step 3：腾讯云/华为云凭据结构在前端同样走 JSON 字符串透传（服务端加密落库）
describe('dnsProviders API（Step 3 · tencent/huawei 凭据透传）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('createDnsProvider 支持 tencent secret_id/secret_key 凭据', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'tenc', provider_type: 'tencent', credentials_json: '{"secret_id":"a","secret_key":"b"}' }
    await createDnsProvider(req)
    expect(api.post).toHaveBeenCalledWith('/dns-providers', req)
  })

  it('createDnsProvider 支持 huawei access_key/secret_key 凭据', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'hw', provider_type: 'huawei', credentials_json: '{"access_key":"a","secret_key":"b"}' }
    await createDnsProvider(req)
    expect(api.post).toHaveBeenCalledWith('/dns-providers', req)
  })
})
