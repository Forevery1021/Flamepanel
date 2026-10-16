import { onBeforeUnmount, onDeactivated, onActivated } from 'vue'
import { connectWithRetry, type WSConnection } from '@/utils/ws'

export interface UseWebSocketOptions {
  onMessage: (data: unknown) => void
  onStatus?: (connected: boolean) => void
  /**
   * 是否自动挂接组件生命周期（卸载 close、keep-alive 失活/激活 pause/resume），默认 true。
   * 端点随响应式状态切换（如 `/ws/metrics?node_id=`）时由调用方在 setup 内统一挂接，
   * 再在 watch 回调里重建实例——Vue 不允许在 setup 之外注册生命周期钩子。
   */
  manageLifecycle?: boolean
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
/** 服务端背压控制帧：`{type:"resync"}`（见 flame-kernel `api/ws_backpressure.rs`） */
export function isResyncFrame(data: unknown): boolean {
  return typeof data === 'object' && data !== null && (data as { type?: unknown }).type === 'resync'
}

class SharedConnectionRegistry {
  private static instance: SharedConnectionRegistry | null = null
  static get() {
    if (!this.instance) this.instance = new SharedConnectionRegistry()
    return this.instance
  }

  private conns = new Map<string, WSConnection>()
  private subscribers = new Map<string, Set<SharedSubscriber>>()

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
        // S-08：服务端判定本连接为慢消费者（连续 lag）时发 {type:"resync"} 并随即关闭。
        // 立即重连（不等 1 s 起步的退避），重连后的 init 帧带完整历史窗口即可补齐曲线；
        // 该控制帧不下发给订阅者（它不是数据）。
        if (isResyncFrame(data)) {
          this.conns.get(url)?.reconnectNow()
          return
        }
        for (const sub of this.subscribers.get(url) ?? []) {
          if (sub.active) sub.onMessage(data)
        }
      },
      onStatus: (connected) => {
        for (const sub of this.subscribers.get(url) ?? []) {
          if (sub.active) sub.onStatus?.(connected)
        }
      },
      // 4003：连接已停止重连；移出注册表，页面重新激活（如权限恢复后再进入）时可重建
      onDenied: () => {
        this.conns.delete(url)
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

  if (options.manageLifecycle !== false) {
    onActivated(() => resume())
    onDeactivated(() => pause())
    onBeforeUnmount(() => close())
  }

  return { connect, close, pause, resume, send, reconnectNow, isActive }
}
