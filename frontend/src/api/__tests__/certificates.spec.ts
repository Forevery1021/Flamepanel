import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：SSL 证书 API 层（Doc/20 §6.1 Stage 2 前端接入）——断言请求路径与方法正确
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  return { default: { get, post, put, delete: del } }
})

import api from '@/api/client'
import {
  listCertificates,
  deleteCertificate,
  issueAcme,
  issueSelfSigned,
  uploadCertificate,
  bindCertificate,
  updateCertificate,
  renewCertificate,
} from '@/api/certificates'

describe('certificates API（SSL 证书）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listCertificates 走 GET /certificates', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { certificates: [], total: 0 } })
    await listCertificates()
    expect(api.get).toHaveBeenCalledWith('/certificates')
  })

  it('issueAcme 走 POST /certificates/issue/acme', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { domains: ['a.com'], dns01: false }
    await issueAcme(req)
    expect(api.post).toHaveBeenCalledWith('/certificates/issue/acme', req)
  })

  it('issueAcme 透传 DNS 厂商账号 ID（Stage 5 泛域名签发）', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { domains: ['*.example.com'], dns01: true, dns_provider_account_id: 5 }
    await issueAcme(req)
    expect(api.post).toHaveBeenCalledWith('/certificates/issue/acme', req)
  })

  it('issueSelfSigned 走 POST /certificates/issue/self-signed', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await issueSelfSigned({ domains: ['a.com'] })
    expect(api.post).toHaveBeenCalledWith('/certificates/issue/self-signed', { domains: ['a.com'] })
  })

  it('uploadCertificate 走 POST /certificates', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { domains: ['a.com'], cert_pem: 'c', key_pem: 'k' }
    await uploadCertificate(req)
    expect(api.post).toHaveBeenCalledWith('/certificates', req)
  })

  it('bindCertificate 走 POST /certificates/:id/bind', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await bindCertificate(3, 7)
    expect(api.post).toHaveBeenCalledWith('/certificates/3/bind', { website_id: 7 })
    // 解绑语义：website_id = null
    await bindCertificate(3, null)
    expect(api.post).toHaveBeenCalledWith('/certificates/3/bind', { website_id: null })
  })

  it('updateCertificate 走 PUT /certificates/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateCertificate(2, { auto_renew: false })
    expect(api.put).toHaveBeenCalledWith('/certificates/2', { auto_renew: false })
  })

  it('deleteCertificate 走 DELETE /certificates/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await deleteCertificate(9)
    expect(api.delete).toHaveBeenCalledWith('/certificates/9')
  })

  it('renewCertificate 走 POST /certificates/{id}/renew', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await renewCertificate(7)
    expect(api.post).toHaveBeenCalledWith('/certificates/7/renew')
  })
})
