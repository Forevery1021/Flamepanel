import { useToast } from 'openvue/usetoast'
import { getErrorMessage, getErrorRequestId } from '@/utils/error'
import type { ToastMessageOptions } from 'openvue/toast'

/**
 * FpToast — 统一操作反馈（消息单例，新消息顶掉旧消息防堆积）。
 * 错误优先展示后端/API 错误消息（复用 getErrorMessage）。
 */
// B6：last 提升为模块级——跨组件共享同一「当前消息」，真正实现新消息顶掉旧消息防堆积
let last: ToastMessageOptions | null = null

export function useFpToast() {
  const toast = useToast()
  function show(message: ToastMessageOptions) {
    if (last) toast.remove(last)
    last = message
    toast.add(message)
  }

  return {
    show,
    success(message: string, title = '') {
      show({ severity: 'success', summary: title, detail: message, life: 3000 })
    },
    error(err: unknown, fallback = '操作失败', title = '') {
      // W0：5xx 附请求编号（语言无关的 "ID:" 前缀），可在「日志中心 → 内核日志」按编号检索；停留更久便于复制
      const rid = getErrorRequestId(err)
      const message = getErrorMessage(err, fallback)
      show({
        severity: 'error',
        summary: title,
        detail: rid ? `${message} (ID: ${rid})` : message,
        life: rid ? 8000 : 4000,
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
