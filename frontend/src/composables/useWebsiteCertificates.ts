import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import {
  listCertificates,
  deleteCertificate,
  issueAcme,
  issueSelfSigned,
  uploadCertificate,
  bindCertificate,
  renewCertificate,
  updateCertificate,
  getCertificateRenewals,
  type Certificate,
  type CertificateRenewalHistory,
} from '@/api/certificates'
import { listDnsProviders } from '@/api/dnsProviders'
import type { DnsProviderAccountDto } from '@/api/generated'

/** ACME 签发载荷（表单编辑态在弹窗组件内） */
export interface AcmeIssuePayload {
  domains: string[]
  dns01: boolean
  webroot?: string
  dns_provider_account_id?: number
}

/**
 * SSL 证书数据层（Doc/23 §3.3 T2-4；主线三 §6.1 Stage 2/3）。
 *
 * WebsitesView 的「SSL 证书」页签此前与网站页签共处一个 1181 行文件，数据侧
 * 混着两族互不相干的接口。本 composable 收敛证书族：
 *
 * - 列表加载（切到证书页签时惰性触发，由视图层 watch 调用 `fetch`）；
 * - 三条签发链路（ACME / 自签 / 上传）与绑定、解绑、删除（后两者带二次确认）；
 * - 手动续签（行内 loading 用 `renewingId` 锁单行）与 `auto_renew` 开关回写；
 * - DNS-01 厂商账号下拉（仅启用项；签发弹窗勾选 DNS-01 时消费）；
 * - S-10 续签记录抽屉：`openHistory` 打开即拉取，`refreshHistory` 重拉（已有内容时不闪骨架）。
 *
 * `handleToggleAutoRenew` 保留「成功后就地回写行数据」的原语义（不再整表重取）。
 */
export function useWebsiteCertificates() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const certificates = ref<Certificate[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const submitting = ref(false)
  const renewingId = ref<number | null>(null)

  // S-10：续签记录抽屉
  const historyVisible = ref(false)
  const historyCert = ref<Certificate | null>(null)
  const history = ref<CertificateRenewalHistory | null>(null)
  const historyFetching = ref(false)
  const historyError = ref<string | null>(null)

  // §6.1 Stage 3：DNS-01 厂商账号下拉（签发 ACME 泛域名时选用）
  const dnsAccounts = ref<DnsProviderAccountDto[]>([])

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      const res = await listCertificates()
      certificates.value = res.data.certificates || []
    } catch {
      error.value = t('common.failed')
    } finally {
      loading.value = false
    }
  }

  /** 拉取 DNS 厂商账号（仅用于下拉，失败静默——不影响 HTTP-01 签发路径） */
  async function loadDnsAccounts() {
    try {
      const res = await listDnsProviders()
      dnsAccounts.value = res.data.filter((a) => a.enabled)
    } catch {
      dnsAccounts.value = []
    }
  }

  /** ACME 签发；成功返回 true */
  async function issueAcmeCert(payload: AcmeIssuePayload): Promise<boolean> {
    submitting.value = true
    try {
      await issueAcme({
        domains: payload.domains,
        dns01: payload.dns01,
        webroot: payload.dns01 ? undefined : payload.webroot || undefined,
        dns_provider_account_id: payload.dns01 ? payload.dns_provider_account_id : undefined,
      })
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 自签证书；成功返回 true */
  async function issueSelfSignedCert(domains: string[]): Promise<boolean> {
    submitting.value = true
    try {
      await issueSelfSigned({ domains })
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 上传第三方证书；成功返回 true */
  async function upload(payload: {
    domains: string[]
    cert_pem: string
    key_pem: string
  }): Promise<boolean> {
    submitting.value = true
    try {
      await uploadCertificate(payload)
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 绑定 / 解绑网站（website_id = null 解绑）；成功返回 true */
  async function bind(id: number, websiteId: number | null): Promise<boolean> {
    submitting.value = true
    try {
      await bindCertificate(id, websiteId)
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 手动续签（单行 loading；仅 ACME 来源可用，其余后端返回 400） */
  async function renew(row: Certificate) {
    renewingId.value = row.id
    try {
      await renewCertificate(row.id)
      toast.success(t('certificate.renewSuccess'))
      await fetch()
    } catch {
      toast.error(t('common.failed'))
    } finally {
      renewingId.value = null
    }
  }

  /** auto_renew 开关：成功后就地回写行数据（不整表重取） */
  async function toggleAutoRenew(row: Certificate, enabled: boolean) {
    try {
      await updateCertificate(row.id, { auto_renew: enabled })
      row.auto_renew = enabled
      toast.success(t('common.success'))
    } catch {
      toast.error(t('common.failed'))
    }
  }

  async function refreshHistory() {
    const cert = historyCert.value
    if (!cert) return
    historyFetching.value = true
    historyError.value = null
    try {
      const res = await getCertificateRenewals(cert.id)
      // 抽屉已切到别的证书：丢弃过期响应
      if (historyCert.value?.id === cert.id) history.value = res.data
    } catch {
      if (historyCert.value?.id === cert.id) historyError.value = t('common.failed')
    } finally {
      historyFetching.value = false
    }
  }

  /** 打开续签记录抽屉（换证书时清空旧内容，避免闪现上一张的时间线） */
  function openHistory(row: Certificate) {
    if (historyCert.value?.id !== row.id) history.value = null
    historyCert.value = row
    historyVisible.value = true
    void refreshHistory()
  }

  function unbind(row: Certificate) {
    confirmAction({
      message: t('certificate.unbind'),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await bindCertificate(row.id, null)
          toast.success(t('common.success'))
          await fetch()
        } catch {
          toast.error(t('common.failed'))
        }
      },
    })
  }

  function remove(row: Certificate) {
    confirmAction({
      message: t('certificate.deleteConfirm'),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await deleteCertificate(row.id)
          toast.success(t('common.success'))
          await fetch()
        } catch {
          toast.error(t('common.failed'))
        }
      },
    })
  }

  return {
    certificates,
    loading,
    error,
    submitting,
    renewingId,
    dnsAccounts,
    fetch,
    loadDnsAccounts,
    issueAcmeCert,
    issueSelfSignedCert,
    upload,
    bind,
    renew,
    toggleAutoRenew,
    unbind,
    remove,
    historyVisible,
    historyCert,
    history,
    historyFetching,
    historyError,
    openHistory,
    refreshHistory,
  }
}
