import { onBeforeUnmount, onDeactivated, onActivated } from 'vue'
import { connectWithRetry, type WSConnection } from '@/utils/ws'

export interface UseWebSocketOptions {
  onMessage: (data: unknown) => void
  onStatus?: (connected: boolean) => void
  /** 手动重连时立即重连 */
  manual?: boolean
}

export interface UseWebSocket {
  /** 建立连接（首次在 onMounted 调用；keep-alive 恢复时由 resume 触发） */
  connect: () => void
  /** 关闭连接并停止重连 */
  close: () => void
  /** 暂停（keep-alive 失活）：关闭连接 */
  pause: () => void
  /** 恢复（keep-alive 激活）：重新连接 */
  resume: () => void
  /** 发送消息（未连接时返回 false） */
  send: (data: string) => boolean
  /** 立即重连 */
  reconnectNow: () => void
  /** 是否处于激活态 */
  isActive: () => boolean
}

interface SharedSubscriber {
  onMessage: (data: unknown) => void
  onStatus?: (connected: boolean) => void
  /** 是否处于激活态（keep-alive 失活时暂时不参与引用计数） */
  active: boolean
}

/**
 * 端点级共享连接注册表。
 *
 * 以端点 URL 为 key，维护同一端点的连接与订阅者列表：
 * - 订阅者激活数量 > 0 时建立连接，归零时关闭（避免 Dashboard/Health 同开 /ws/metrics 双连接）
 * - 同一端点的多个页面各自持有 onMessage/onStatus 回调，连接建立后统一分发
 */
class SharedConnectionRegistry {
  private static instance: SharedConnectionRegistry | null = null
  static get() {
    if (!this.instance) this.instance = new SharedConnectionRegistry()
    return this.instance
  }

  private conns = new Map<string, WSConnection>()
  private subscribers = new Map<string, Set<SharedSubscriber>>()
  /** 手动重连请求标记（键为 url） */
  private manualRetryFlags = new Map<string, boolean>()

  private activeCount(url: string): number {
    let n = 0
    for (const sub of this.subscribers.get(url) ?? []) {
      if (sub.active) n++
    }
    return n
  }

  /** 订阅端点：返回订阅令牌 */
  subscribe(url: string, sub: SharedSubscriber): () => void {
    if (!this.subscribers.has(url)) this.subscribers.set(url, new Set())
    this.subscribers.get(url)!.add(sub)
    // 若当前无连接且已有其他激活订阅，则建立共享连接
    if (!this.conns.has(url) && this.activeCount(url) > 0) {
      this.ensureConnection(url)
    }
    return () => this.unsubscribe(url, sub)
  }

  private unsubscribe(url: string, sub: SharedSubscriber) {
    const set = this.subscribers.get(url)
    if (!set) return
    set.delete(sub)
    if (set.size === 0) {
      this.closeConnection(url)
    }
  }

  /** 订阅者激活状态变化时重新评估连接生命周期 */
  reevaluate(url: string) {
    const hasActive = this.activeCount(url) > 0
    if (hasActive && !this.conns.has(url)) {
      this.ensureConnection(url)
    } else if (!hasActive && this.conns.has(url)) {
      this.closeConnection(url)
    }
  }

  private ensureConnection(url: string) {
    if (this.conns.has(url)) return
    const conn = connectWithRetry(url, {
      onMessage: (data) => {
        for (const sub of this.subscribers.get(url) ?? []) {
          if (sub.active) sub.onMessage(data)
        }
      },
      onStatus: (connected) => {
        for (const sub of this.subscribers.get(url) ?? []) {
          if (sub.active) sub.onStatus?.(connected)
        }
      },
    })
    this.conns.set(url, conn)
  }

  private closeConnection(url: string) {
    this.conns.get(url)?.close()
    this.conns.delete(url)
  }

  send(url: string, data: string): boolean {
    return this.conns.get(url)?.send(data) ?? false
  }

  reconnectNow(url: string) {
    const conn = this.conns.get(url)
    if (conn) conn.reconnectNow()
  }
}

/**
 * useSharedWebSocket — 端点级共享 WebSocket（同一端点仅一条连接）。
 *
 * 与 useWebSocket 保持 API 兼容，但内部通过注册表做引用计数 + 订阅分发，
 * 使多个组件共享同一端点连接（如 Dashboard 与 Health 同时激活 /ws/metrics）。
 * - 组件卸载自动 unsubscribe（引用计数减一）
 * - keep-alive 失活时暂停订阅（不占用连接计数），恢复时重新激活
 */
export function useSharedWebSocket(url: string, options: UseWebSocketOptions): UseWebSocket {
  const registry = SharedConnectionRegistry.get()
  const sub: SharedSubscriber = {
    onMessage: options.onMessage,
    onStatus: options.onStatus,
    active: true,
  }
  let unsubscribe: (() => void) | null = null

  function connect() {
    if (unsubscribe) return
    sub.active = true
    unsubscribe = registry.subscribe(url, sub)
    // 订阅后重新评估连接生命周期（激活数 > 0 则建立共享连接）
    registry.reevaluate(url)
  }

  function close() {
    if (!unsubscribe) return
    sub.active = false
    registry.reevaluate(url)
    unsubscribe()
    unsubscribe = null
  }

  function pause() {
    if (!unsubscribe) return
    sub.active = false
    registry.reevaluate(url)
  }

  function resume() {
    if (!unsubscribe) return
    sub.active = true
    registry.reevaluate(url)
  }

  function send(data: string): boolean {
    return registry.send(url, data)
  }

  function reconnectNow() {
    registry.reconnectNow(url)
  }

  function isActive() {
    return sub.active
  }

  onActivated(() => resume())
  onDeactivated(() => pause())
  onBeforeUnmount(() => close())

  return { connect, close, pause, resume, send, reconnectNow, isActive }
}
