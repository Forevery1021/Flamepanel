/**
 * 全局轻量通知通道（「非组件上下文」→ UI）。
 *
 * 路由守卫（`router/index.ts`）运行在组件树之外，没有 `setup()` 上下文——
 * 直接调用 `useFpToast()` 会因 `inject` 失败而抛错（「No PrimeVue Toast provided!」）。
 * 故本模块提供一个最小 pub/sub + 队列：
 *
 * - 有订阅者（布局已挂载）→ 立即投递；
 * - 无订阅者（刷新页面时守卫先于挂载运行）→ 暂存，订阅建立后自动补投，
 *   避免「无权限提示」在硬刷场景静默丢失。
 *
 * 消息只携带 **i18n key 与参数**，由订阅方取词——守卫侧语言包可能尚未装载，
 * 提前固化译文会上屏裸 key 或错误语言。
 */

export type NoticeSeverity = 'info' | 'warn' | 'error'

export interface NoticeMessage {
  /** i18n key（订阅方负责取词） */
  key: string
  /** 取词参数 */
  params?: Record<string, unknown>
  severity: NoticeSeverity
}

export type NoticeHandler = (message: NoticeMessage) => void

let handler: NoticeHandler | null = null
let queue: NoticeMessage[] = []

/** 投递一条通知：有订阅者直投，否则暂存待补投 */
export function emitNotice(message: NoticeMessage) {
  if (handler) {
    handler(message)
    return
  }
  queue.push(message)
}

/**
 * 建立订阅（返回取消订阅函数）。
 * 建立时立即补投此前暂存的消息，保证「先守卫后挂载」的顺序不丢提示。
 */
export function onNotice(fn: NoticeHandler): () => void {
  handler = fn
  const pending = queue
  queue = []
  for (const message of pending) fn(message)
  return () => {
    if (handler === fn) handler = null
  }
}

/** 测试专用：清空订阅与队列（模块级状态，避免用例间互相污染） */
export function resetNoticeChannel() {
  handler = null
  queue = []
}
