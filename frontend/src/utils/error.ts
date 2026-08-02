import { useI18n } from 'vue-i18n'
import type { ApiError } from '@/api/client'

/**
 * 统一错误消息：
 * 1. 后端返回稳定错误码（error 字段）时，优先查 i18n `common.error.<code>` 本地化文案
 * 2. 无错误码或未配置文案时，回退到后端 message
 * 3. 最后回退到调用方提供的默认文案
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
  const resp = (e as { response?: { data?: { message?: string } } }).response
  return resp?.data?.message || (e instanceof Error ? e.message : fallback)
}
