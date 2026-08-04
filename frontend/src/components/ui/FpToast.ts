import { useToast } from 'openvue/usetoast'
import { getErrorMessage } from '@/utils/error'
import type { ToastMessageOptions } from 'openvue/toast'

/**
 * FpToast — 统一操作反馈（消息单例，新消息顶掉旧消息防堆积）。
 * 错误优先展示后端/API 错误消息（复用 getErrorMessage）。
 */
export function useFpToast() {
  const toast = useToast()
  let last: ToastMessageOptions | null = null

  function show(message: ToastMessageOptions) {
    if (last) toast.remove(last)
    last = message
    toast.add(message)
  }

  return {
    success(message: string, title = '') {
      show({ severity: 'success', summary: title, detail: message, life: 3000 })
    },
    error(err: unknown, fallback = '操作失败', title = '') {
      show({
        severity: 'error',
        summary: title,
        detail: getErrorMessage(err, fallback),
        life: 4000,
      })
    },
    warning(message: string, title = '') {
      show({ severity: 'warn', summary: title, detail: message, life: 3500 })
    },
    info(message: string, title = '') {
      show({ severity: 'info', summary: title, detail: message, life: 3000 })
    },
  }
}
