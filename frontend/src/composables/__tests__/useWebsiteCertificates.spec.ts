import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站的数据层单测（`WebsitesView` → `useWebsiteCertificates`）。
 *
 * SSL 证书页签的请求编排此前与网站页签混在同一文件。拆出后本 spec 看住四件事：
 * ① 列表加载的三态（含失败时落 error 文案、loading 复位）；
 * ② 三条签发链路的载荷组装（**HTTP-01 与 DNS-01 互斥**：选 DNS-01 时
 *    `webroot` 必须不传、未选厂商账号时也不传 undefined 以外的值）；
 * ③ 绑定 / 解绑 / 删除的确认分支（后两者**必经二次确认**，不得直接发请求）；
 * ④ `auto_renew` 开关的成功回写与失败广播、续签的单行 loading 锁。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listCertificates: vi.fn(),
    deleteCertificate: vi.fn(),
    issueAcme: vi.fn(),
    issueSelfSigned: vi.fn(),
    uploadCertificate: vi.fn(),
    bindCertificate: vi.fn(),
    renewCertificate: vi.fn(),
    updateCertificate: vi.fn(),
    listDnsProviders: vi.fn(),
    getCertificateRenewals: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; message: string } | null },
}))

vi.mock('@/api/certificates', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/certificates')>()
  return { ...actual, ...api }
})
vi.mock('@/api/dnsProviders', () => ({ listDnsProviders: api.listDnsProviders }))
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useWebsiteCertificates } from '@/composables/useWebsiteCertificates'

const messages = {
  certificate: {
    renewSuccess: '续签成功，证书已更新',
    unbind: '解绑',
    deleteConfirm: '确定删除该证书记录吗？（不删除证书文件）',
  },
  common: { success: '成功', failed: '操作失败', confirmAction: '确定执行此操作吗？' },
}

const cert = (over: Record<string, unknown> = {}) => ({
  id: 1,
  domains: '["example.com"]',
  source: 'letsencrypt',
  expires_at: '2027-01-01T00:00:00Z',
  auto_renew: true,
  website_id: null,
  created_at: '',
  updated_at: '',
  ...over,
})

function mountCerts() {
  const w = mount(
    defineComponent({
      setup() {
        return { certs: useWebsiteCertificates() }
      },
      template: '<div/>',
    }),
    { global: { plugins: [createTestI18n(messages)], directives: { permission: () => {} } } },
  )
  return (w.vm as unknown as { certs: ReturnType<typeof useWebsiteCertificates> }).certs
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listCertificates.mockResolvedValue({ data: { certificates: [cert()], total: 1 } })
  api.issueAcme.mockResolvedValue({ data: cert() })
  api.issueSelfSigned.mockResolvedValue({ data: cert({ source: 'self_signed' }) })
  api.uploadCertificate.mockResolvedValue({ data: cert({ source: 'uploaded' }) })
  api.bindCertificate.mockResolvedValue({ data: cert() })
  api.deleteCertificate.mockResolvedValue({ data: null })
  api.renewCertificate.mockResolvedValue({ data: cert() })
  api.updateCertificate.mockResolvedValue({ data: cert() })
  api.listDnsProviders.mockResolvedValue({ data: [] })
})

describe('useWebsiteCertificates · 列表加载', () => {
  it('fetch 落数据并复位 loading', async () => {
    const certs = mountCerts()
    expect(certs.loading.value).toBe(false)

    await certs.fetch()
    expect(certs.certificates.value).toHaveLength(1)
    expect(certs.loading.value).toBe(false)
    expect(certs.error.value).toBeNull()
  })

  it('响应缺 certificates 字段时降级为空数组（不把 undefined 交给表格）', async () => {
    api.listCertificates.mockResolvedValue({ data: {} })
    const certs = mountCerts()
    await certs.fetch()

    expect(certs.certificates.value).toEqual([])
  })

  it('失败：error 落通用文案且 loading 复位（不卡死转圈）', async () => {
    api.listCertificates.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    await certs.fetch()

    expect(certs.error.value).toBe('操作失败')
    expect(certs.loading.value).toBe(false)
  })

  it('loadDnsAccounts：只保留启用项；失败静默（HTTP-01 路径不受影响）', async () => {
    api.listDnsProviders.mockResolvedValue({
      data: [
        { id: 1, name: 'cf', provider_type: 'cloudflare', enabled: true },
        { id: 2, name: 'ali', provider_type: 'aliyun', enabled: false },
      ],
    })
    const certs = mountCerts()
    await certs.loadDnsAccounts()
    expect(certs.dnsAccounts.value.map((a) => a.id)).toEqual([1])

    api.listDnsProviders.mockRejectedValue(new Error('boom'))
    await certs.loadDnsAccounts()
    expect(certs.dnsAccounts.value).toEqual([])
  })
})

describe('useWebsiteCertificates · 签发载荷（HTTP-01 / DNS-01 互斥）', () => {
  it('HTTP-01：上报 webroot，dns_provider_account_id 不传', async () => {
    const certs = mountCerts()
    expect(
      await certs.issueAcmeCert({
        domains: ['example.com'],
        dns01: false,
        webroot: '/www/example',
      }),
    ).toBe(true)

    expect(api.issueAcme).toHaveBeenCalledWith({
      domains: ['example.com'],
      dns01: false,
      webroot: '/www/example',
      dns_provider_account_id: undefined,
    })
  })

  it('HTTP-01 且 webroot 为空：上报 undefined（后端按默认根处理，不得传空串）', async () => {
    const certs = mountCerts()
    await certs.issueAcmeCert({ domains: ['example.com'], dns01: false, webroot: '' })

    expect(api.issueAcme).toHaveBeenCalledWith(expect.objectContaining({ webroot: undefined }))
  })

  it('DNS-01：上报厂商账号，**webroot 必须不传**（两种验证方式互斥）', async () => {
    const certs = mountCerts()
    await certs.issueAcmeCert({
      domains: ['*.example.com'],
      dns01: true,
      webroot: '/www/example', // 即便误带也不得透传
      dns_provider_account_id: 3,
    })

    expect(api.issueAcme).toHaveBeenCalledWith({
      domains: ['*.example.com'],
      dns01: true,
      webroot: undefined,
      dns_provider_account_id: 3,
    })
  })

  it('签发成功后刷新列表并返回 true', async () => {
    const certs = mountCerts()
    await certs.issueAcmeCert({ domains: ['a.com'], dns01: false })

    expect(api.listCertificates).toHaveBeenCalled()
    expect(toast.success).toHaveBeenCalledWith('成功')
  })

  it('签发失败：返回 false（调用方不关弹窗，输入保留）', async () => {
    api.issueAcme.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()

    expect(await certs.issueAcmeCert({ domains: ['a.com'], dns01: false })).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('自签：上报 SAN 列表并刷新', async () => {
    const certs = mountCerts()
    expect(await certs.issueSelfSignedCert(['intranet.local', '*.intranet.local'])).toBe(true)
    expect(api.issueSelfSigned).toHaveBeenCalledWith({
      domains: ['intranet.local', '*.intranet.local'],
    })
  })

  it('上传：PEM 原样透传（不做裁剪，证书链首尾空白有意义）', async () => {
    const certs = mountCerts()
    expect(
      await certs.upload({
        domains: ['uploaded.com'],
        cert_pem: '\n-----BEGIN CERTIFICATE-----\n',
        key_pem: '-----BEGIN PRIVATE KEY-----\n',
      }),
    ).toBe(true)
    expect(api.uploadCertificate).toHaveBeenCalledWith({
      domains: ['uploaded.com'],
      cert_pem: '\n-----BEGIN CERTIFICATE-----\n',
      key_pem: '-----BEGIN PRIVATE KEY-----\n',
    })
  })

  it('上传失败：返回 false（调用方不关弹窗）', async () => {
    api.uploadCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    expect(await certs.upload({ domains: ['a.com'], cert_pem: 'C', key_pem: 'K' })).toBe(false)
  })

  it('自签失败：返回 false 且 toast 报错', async () => {
    api.issueSelfSigned.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    expect(await certs.issueSelfSignedCert(['a.com'])).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('绑定失败：返回 false 且 toast 报错', async () => {
    api.bindCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    expect(await certs.bind(7, 3)).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('删除失败：toast 报错，列表不崩', async () => {
    api.deleteCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    certs.remove(cert({ id: 8 }))
    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))

    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useWebsiteCertificates · 绑定 / 解绑 / 删除', () => {
  it('bind：直接发请求（绑定动作本身即幂等，无二次确认）', async () => {
    const certs = mountCerts()
    expect(await certs.bind(7, 3)).toBe(true)
    expect(api.bindCertificate).toHaveBeenCalledWith(7, 3)
    expect(confirmState.last).toBeNull()
  })

  it('unbind：必须先二次确认，确认后才发请求（解除绑定影响线上 TLS）', async () => {
    const certs = mountCerts()
    certs.unbind(cert({ id: 7 }))
    expect(confirmState.last?.message).toBe('解绑')
    expect(api.bindCertificate).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.bindCertificate).toHaveBeenCalledWith(7, null)
  })

  it('remove：必须先二次确认（文案说明只删记录不删文件）', async () => {
    const certs = mountCerts()
    certs.remove(cert({ id: 8 }))
    expect(confirmState.last?.message).toContain('不删除证书文件')
    expect(api.deleteCertificate).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.deleteCertificate).toHaveBeenCalledWith(8)
  })

  it('unbind 失败：toast 报错，列表不崩', async () => {
    api.bindCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    certs.unbind(cert())
    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))

    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useWebsiteCertificates · 续签与 auto_renew', () => {
  it('renew：单行 loading（renewingId 锁当前行），成功后刷新并复位', async () => {
    let release: (v: unknown) => void = () => {}
    api.renewCertificate.mockReturnValue(new Promise((r) => (release = r)))
    const certs = mountCerts()

    const pending = certs.renew(cert({ id: 9 }))
    await new Promise((r) => setTimeout(r, 5))
    expect(certs.renewingId.value).toBe(9)

    release({ data: cert() })
    await pending
    expect(certs.renewingId.value).toBeNull()
    expect(toast.success).toHaveBeenCalledWith('续签成功，证书已更新')
  })

  it('renew 失败：renewingId 也必须复位（否则按钮永久转圈）', async () => {
    api.renewCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    await certs.renew(cert({ id: 9 }))

    expect(certs.renewingId.value).toBeNull()
    expect(toast.error).toHaveBeenCalled()
  })

  it('toggleAutoRenew 成功：就地对行数据回写（不再整表重取）', async () => {
    const certs = mountCerts()
    const row = cert({ id: 5, auto_renew: true })
    const before = api.listCertificates.mock.calls.length

    await certs.toggleAutoRenew(row as never, false)
    expect(api.updateCertificate).toHaveBeenCalledWith(5, { auto_renew: false })
    expect(row.auto_renew).toBe(false)
    expect(api.listCertificates.mock.calls.length).toBe(before)
  })

  it('toggleAutoRenew 失败：不回写行数据并报错（界面保持服务端真值）', async () => {
    api.updateCertificate.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    const row = cert({ id: 5, auto_renew: true })

    await certs.toggleAutoRenew(row as never, false)
    expect(row.auto_renew).toBe(true)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useWebsiteCertificates · S-10 续签记录抽屉', () => {
  const hist = (id: number) => ({ certificate_id: id, entries: [], truncated: false })

  it('openHistory：打开抽屉并拉取该证书的续签记录', async () => {
    api.getCertificateRenewals.mockResolvedValue({ data: hist(7) })
    const certs = mountCerts()
    certs.openHistory(cert({ id: 7 }) as never)
    expect(certs.historyVisible.value).toBe(true)
    expect(certs.historyFetching.value).toBe(true)
    await vi.waitFor(() => expect(certs.historyFetching.value).toBe(false))
    expect(api.getCertificateRenewals).toHaveBeenCalledWith(7)
    expect(certs.history.value?.certificate_id).toBe(7)
    expect(certs.historyError.value).toBeNull()
  })

  it('换证书时先清空旧时间线；迟到的旧响应被丢弃', async () => {
    let resolveOld!: (v: unknown) => void
    api.getCertificateRenewals
      .mockImplementationOnce(() => new Promise((r) => (resolveOld = r)))
      .mockResolvedValueOnce({ data: hist(2) })
    const certs = mountCerts()
    certs.openHistory(cert({ id: 1 }) as never)
    certs.openHistory(cert({ id: 2 }) as never)
    await vi.waitFor(() => expect(certs.history.value?.certificate_id).toBe(2))
    resolveOld({ data: hist(1) })
    await new Promise((r) => setTimeout(r, 0))
    expect(certs.history.value?.certificate_id).toBe(2)
  })

  it('拉取失败：落 error 文案、fetching 复位', async () => {
    api.getCertificateRenewals.mockRejectedValue(new Error('boom'))
    const certs = mountCerts()
    certs.openHistory(cert({ id: 3 }) as never)
    await vi.waitFor(() => expect(certs.historyFetching.value).toBe(false))
    expect(certs.historyError.value).toBe('操作失败')
  })
})
