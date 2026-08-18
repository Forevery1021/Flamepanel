import { STORAGE_KEYS } from './storage'

/**
 * WebSocket 带自动重连的连接工具（指数退避）
 * 用法:
 *   const conn = connectWithRetry('/ws/metrics', {
 *     onMessage: (data) => {...},
 *     onStatus: (connected) => {...},
 *   })
 *   conn.close()  // 停止重连并关闭
 *
 * 连接会自动附加当前登录 token（`?token=<access_token>`），
 * 后端 WS 握手时校验（Stage4.1：WS 鉴权加固）。
 *
 * A2：引入代际（generation）计数防止双连接——
 * `reconnectNow()` 先递增代际再 close+connect，旧 socket 的
 * onclose/onerror 回调因代际不匹配被忽略，不会再次调度重连。
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
}

const MAX_RETRY_MS = 30000

/** 从 localStorage 读取当前 access token */
function currentToken(): string {
  return localStorage.getItem(STORAGE_KEYS.token) || ''
}

/** 拼接 `?token=` 查询参数（URL 已带查询时用 & 连接） */
function withToken(url: string, token: string): string {
  if (!token) return url
  const sep = url.includes('?') ? '&' : '?'
  return `${url}${sep}token=${encodeURIComponent(token)}`
}

export function connectWithRetry(url: string, options: WSOptions): WSConnection {
  let ws: WebSocket | null = null
  let closed = false
  let retryMs = 1000
  let retryTimer: number | null = null
  // A2：代际计数。每次 connect 递增；旧 socket 的回调据此判失效，避免双连接。
  let currentGen = 0

  function scheduleReconnect() {
    if (closed) return
    options.onStatus?.(false)
    retryTimer = window.setTimeout(() => {
      retryTimer = null
      connect()
    }, retryMs)
    // 指数退避：1s → 2s → 4s → ... 上限 30s
    retryMs = Math.min(retryMs * 2, MAX_RETRY_MS)
  }

  function connect() {
    if (closed) return
    const gen = ++currentGen
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    // Stage4.1：WS 鉴权 — 握手时携带当前 access token
    const socket = new WebSocket(`${protocol}//${location.host}${withToken(url, currentToken())}`)
    ws = socket

    socket.onopen = () => {
      if (gen !== currentGen) return
      retryMs = 1000
      options.onStatus?.(true)
    }

    socket.onmessage = (ev: MessageEvent) => {
      if (gen !== currentGen) return
      try {
        options.onMessage(JSON.parse(ev.data))
      } catch {
        options.onMessage(ev.data)
      }
    }

    socket.onclose = () => {
      // 旧代际 socket 关闭不再触发重连（防 reconnectNow 双连接回归）
      if (gen !== currentGen) return
      if (!closed) scheduleReconnect()
    }

    socket.onerror = () => {
      if (gen !== currentGen) return
      socket.close()
    }
  }

  connect()

  return {
    close() {
      closed = true
      currentGen++
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
      // 递增代际使旧 socket 的回调全部失效，再关闭并立即重建，保证仅一条活跃连接
      currentGen++
      if (retryTimer !== null) {
        clearTimeout(retryTimer)
        retryTimer = null
      }
      ws?.close()
      retryMs = 1000
      connect()
    },
  }
}