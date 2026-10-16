import api from './client'
import type { Certificate, CertificateRenewalHistory, RenewalEntry } from '@/api/generated'

export type { Certificate, CertificateRenewalHistory, RenewalEntry }

/** 来源是否支持自动续签（仅 ACME；与后端 `Certificate::supports_auto_renew` 同源） */
export function supportsAutoRenew(cert: Pick<Certificate, 'source'>): boolean {
  return cert.source === 'letsencrypt' || cert.source === 'zerossl'
}

/** 证书列表 */
export function listCertificates() {
  return api.get<{ certificates: Certificate[]; total: number }>('/certificates')
}

/** 删除证书记录（不删除落盘文件） */
export function deleteCertificate(id: number) {
  return api.delete(`/certificates/${id}`)
}

/** ACME 签发（Let's Encrypt 等；HTTP-01 / DNS-01） */
export function issueAcme(req: {
  domains: string[]
  directory_url?: string
  dns01?: boolean
  webroot?: string
  website_id?: number | null
  /** DNS-01 厂商账号 ID（泛域名/声明厂商时必填） */
  dns_provider_account_id?: number | null
}) {
  return api.post<Certificate>('/certificates/issue/acme', req)
}

/** 自签证书（内网/无公网场景） */
export function issueSelfSigned(req: { domains: string[]; website_id?: number | null }) {
  return api.post<Certificate>('/certificates/issue/self-signed', req)
}

/** 上传第三方证书 */
export function uploadCertificate(req: {
  domains: string[]
  cert_pem: string
  key_pem: string
  website_id?: number | null
}) {
  return api.post<Certificate>('/certificates', req)
}

/** 证书绑定/解绑网站（website_id = null 解绑） */
export function bindCertificate(id: number, website_id: number | null) {
  return api.post<Certificate>(`/certificates/${id}/bind`, { website_id })
}

/** 手动触发续签（ACME 来源；自签/上传返回 400） */
export function renewCertificate(id: number) {
  return api.post<Certificate>(`/certificates/${id}/renew`)
}

/** 更新证书（auto_renew / website_id） */
export function updateCertificate(
  id: number,
  req: { auto_renew?: boolean; website_id?: number | null },
) {
  return api.put<Certificate>(`/certificates/${id}`, req)
}

/** 续签历史（S-10）：签发 / 自动续签成功与失败 / 到期预警时间线，最新在前 */
export function getCertificateRenewals(id: number, limit?: number) {
  return api.get<CertificateRenewalHistory>(`/certificates/${id}/renewals`, {
    params: limit ? { limit } : undefined,
  })
}
