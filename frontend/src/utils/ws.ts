/**
 * WebSocket 带自动重连的连接工具（指数退避）
 * 用法:
 *   const conn = connectWithRetry('/ws/metrics', {
 *     onMessage: (data) => {...},
 *     onStatus: (connected) => {...},
 *   })
 *   conn.close()  // 停止重连并关闭
 */
export interface WSConnection {
  close: () => void
  send: (data: string) => boolean
  reconnectNow: () => void
}

interface WSOptions {
  onMessage: (data: unknown) => void
  /** 连接状态变化回调（true=已连接，false=断开/重连中） */
  onStatus?: (connected: boolean) => void
  /** 手动重连按钮触发时设为 true，跳过退避立即重连 */
  manual?: boolean
}

const MAX_RETRY_MS = 30000

export function connectWithRetry(url: string, options: WSOptions): WSConnection {
  let ws: WebSocket | null = null
  let closed = false
  let retryMs = 1000
  let retryTimer: number | null = null
  let manualRetry = options.manual ?? false

  function scheduleReconnect() {
    if (closed) return
    options.onStatus?.(false)
    retryTimer = window.setTimeout(() => {
      retryTimer = null
      connect()
    }, manualRetry ? 0 : retryMs)
    // 指数退避：1s → 2s → 4s → ... 上限 30s
    if (!manualRetry) retryMs = Math.min(retryMs * 2, MAX_RETRY_MS)
  }

  function connect() {
    if (closed) return
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    ws = new WebSocket(`${protocol}//${location.host}${url}`)

    ws.onopen = () => {
      retryMs = 1000
      options.onStatus?.(true)
    }

    ws.onmessage = (ev: MessageEvent) => {
      try {
        options.onMessage(JSON.parse(ev.data))
      } catch {
        options.onMessage(ev.data)
      }
    }

    ws.onclose = () => {
      if (!closed) scheduleReconnect()
    }

    ws.onerror = () => {
      ws?.close()
    }
  }

  connect()

  return {
    close() {
      closed = true
      if (retryTimer !== null) clearTimeout(retryTimer)
      ws?.close()
    },
    send(data: string) {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(data)
        return true
      }
      return false
    },
    reconnectNow() {
      manualRetry = true
      if (retryTimer !== null) {
        clearTimeout(retryTimer)
        retryTimer = null
      }
      ws?.close()
      connect()
    },
  }
}
