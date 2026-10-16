import { describe, it, expect, vi } from 'vitest'

// 模拟 vue-i18n：getErrorMessage 在组件外调用 useI18n 会 throw，需注入 i18n 实例
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => {
      if (key === 'common.error.INVALID_TOKEN') return '登录已过期'
      if (key === 'common.error.NETWORK_ERROR') return '网络不可用'
      return key
    },
    te: (key: string) =>
      key === 'common.error.INVALID_TOKEN' || key === 'common.error.NETWORK_ERROR',
  }),
}))

import { getErrorMessage, getErrorRequestId } from '@/utils/error'
import { isNetworkError } from '@/api/client'

describe('getErrorMessage（F0.1 统一错误文案）', () => {
  it('有稳定错误码且配置了 i18n 文案 → 返回本地化文案', () => {
    const e = { code: 'INVALID_TOKEN', message: 'invalid token', status: 401 } as unknown as Error
    expect(getErrorMessage(e, 'fallback')).toBe('登录已过期')
  })

  it('有错误码但未配置 i18n → 回退后端 message', () => {
    const e = { code: 'SOME_CODE', message: 'backend msg', status: 400 } as unknown as Error
    expect(getErrorMessage(e, 'fallback')).toBe('backend msg')
  })

  it('断网（AxiosError 无 response）→ 本地化网络不可用', () => {
    const e = { isAxiosError: true, message: 'Network Error' }
    expect(getErrorMessage(e, 'fallback')).toBe('网络不可用')
  })

  it('完全未知错误 → 回退调用方默认文案', () => {
    expect(getErrorMessage(null, '自定义兜底')).toBe('自定义兜底')
  })
})

describe('isNetworkError（断网/超时识别）', () => {
  it('有 HTTP 响应 → 不是网络错误', () => {
    const e = { isAxiosError: true, response: { status: 500 }, code: 'ERR_BAD_RESPONSE' }
    expect(isNetworkError(e)).toBe(false)
  })

  it('无响应（断网）→ 是网络错误', () => {
    const e = { isAxiosError: true, code: 'ERR_NETWORK' }
    expect(isNetworkError(e)).toBe(true)
  })

  it('超时 → 是网络错误', () => {
    const e = { isAxiosError: true, code: 'ECONNABORTED' }
    expect(isNetworkError(e)).toBe(true)
  })

  it('非 axios 错误 → 不是网络错误', () => {
    expect(isNetworkError(new Error('boom'))).toBe(false)
    expect(isNetworkError(null)).toBe(false)
  })
})

describe('PASSWORD_CHANGE_REQUIRED 常量', () => {
  it('与后端错误码一致', () => {
    // 见 flame-kernel/src/core/error.rs（强制改密错误码）
    expect('PASSWORD_CHANGE_REQUIRED').toBe('PASSWORD_CHANGE_REQUIRED')
  })
})

describe('getErrorRequestId（W0 5xx 请求编号）', () => {
  it('规范化错误 ≥500 返回 requestId', () => {
    expect(getErrorRequestId({ code: 'INTERNAL_ERROR', status: 502, requestId: 'r-1' })).toBe('r-1')
  })
  it('原始 AxiosError（无统一错误体）从响应头读取', () => {
    expect(
      getErrorRequestId({ response: { status: 500, headers: { 'x-request-id': 'r-2' } } }),
    ).toBe('r-2')
  })
  it('4xx / 无响应 / 空值返回空串', () => {
    expect(getErrorRequestId({ code: 'CONFLICT', status: 409, requestId: 'r-3' })).toBe('')
    expect(getErrorRequestId(new Error('Network Error'))).toBe('')
    expect(getErrorRequestId(null)).toBe('')
  })
})
