import { useI18n } from 'vue-i18n'
import type { ApiError } from '@/api/client'

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
      return '网络不可用，请检查网络连接'
    }
  }
  return resp?.data?.message || (e instanceof Error ? e.message : fallback)
}
