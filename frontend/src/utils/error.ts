import { useI18n } from 'vue-i18n'
import type { ApiError } from '@/api/client'

/** B1：i18n 不可用时的网络错误最小内联兜底（按浏览器语言三选一） */
function networkFallback(): string {
  const lang = (typeof navigator !== 'undefined' && navigator.language) || ''
  if (lang.toLowerCase().startsWith('ja'))
    return 'ネットワークに接続できません。接続を確認してください'
  if (lang.toLowerCase().startsWith('zh')) return '网络不可用，请检查网络连接'
  return 'Network unavailable. Please check your connection.'
}

/**
 * 后端错误载荷中的 `detail` 字段（FlamePanel 部分端点用它承载具体的失败原因，
 * 如安装/卸载失败的真实原因）；无该字段时返回空串，交由调用方回退通用文案。
 */
export function getErrorDetail(e: unknown): string {
  const detail = (e as { response?: { data?: { detail?: string } } } | null)?.response?.data?.detail
  return typeof detail === 'string' ? detail : ''
}

/**
 * 服务端错误（5xx）的请求编号：来自响应头 `X-Request-Id`，内核日志中同名字段可检索。
 * 4xx 属于用户可自行修正的错误，不附带编号以免干扰。
 */
export function getErrorRequestId(e: unknown): string {
  const normalized = e as
    | (Partial<ApiError> & { response?: { status?: number; headers?: Record<string, unknown> } })
    | null
  if (!normalized) return ''
  const status = normalized.status ?? normalized.response?.status ?? 0
  if (status < 500) return ''
  const rid = normalized.requestId ?? normalized.response?.headers?.['x-request-id']
  return typeof rid === 'string' ? rid : ''
}

/**
 * 统一错误消息：
 * 1. 后端返回稳定错误码（error 字段）时，优先查 i18n `common.error.<code>` 本地化文案
 * 2. 无错误码或未配置文案时，回退到后端 message
 * 3. 断网/超时等无响应错误 → 本地化「网络不可用」文案
 * 4. 最后回退到调用方提供的默认文案
 */
export function getErrorMessage(e: unknown, fallback: string): string {
  const normalized = e as Partial<ApiError> | null
  if (normalized?.code) {
    try {
      const { t, te } = useI18n()
      const key = `common.error.${normalized.code}`
      if (te(key)) {
        return t(key)
      }
    } catch {
      // i18n 实例不可用（非组件上下文）时忽略
    }
    if (normalized.message) {
      return normalized.message
    }
  }
  // 断网 / 超时 / 服务未启动：没有 HTTP 响应体
  const resp = (e as { response?: { data?: { message?: string } } } | null)?.response
  if (!resp) {
    const err = e as { isAxiosError?: boolean; message?: string } | null
    if (err?.isAxiosError || err?.message === 'Network Error') {
      try {
        const { t, te } = useI18n()
        if (te('common.error.NETWORK_ERROR')) return t('common.error.NETWORK_ERROR')
      } catch {
        // ignore
      }
      // B1：i18n 不可用时的最小内联兜底——按浏览器语言三选一，避免英文/日文用户看到中文
      return networkFallback()
    }
  }
  return resp?.data?.message || (e instanceof Error ? e.message : fallback)
}
