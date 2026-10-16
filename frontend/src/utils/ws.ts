import { STORAGE_KEYS } from './storage'
import { getRefreshedToken } from '@/api/client' // F-33: 重连前刷新可能已过期的 access token

/**
 * WebSocket 带自动重连的连接工具（指数退避）
 * 用法:
 *   const conn = connectWithRetry('/ws/metrics', {
 *     onMessage: (data) => {...},
 *     onStatus: (connected) => {...},
 *   })
 *   conn.close()  // 停止重连并关闭
 *
 * 连接建立后首帧发送当前登录 token（`{"type":"auth","token":...}`）进行鉴权
 * （R2：WS 首帧鉴权，避免 access token 出现在 URL query 而泄露到浏览器历史/代理/Web 日志）。
 *
 * A2：引入代际（generation）计数防止双连接——
 * `reconnectNow()` 先递增代际再 close+connect，旧 socket 的
 * onclose/onerror 回调因代际不匹配被忽略，不会再次调度重连。
 *
 * 会话控制（与后端 `api/ws_auth.rs` 同口径）：
 * - 关闭码 `4001`（未认证 / 会话过期）→ 强制刷新 access token 后重连；刷新失败由 client 回登录页；
 * - 关闭码 `4003`（无权限，如角色被降级）→ 停止重连，经 `onDenied` 通知页面；
 * - 关闭码 `4004`（订阅对象不存在，如 `/ws/metrics?node_id=` 指向已删除节点）→ 同样停止重连；
 * - 连接期间每 `RENEW_INTERVAL_MS` 检查一次，token 临期先刷新，令牌有变化即发带内续期帧
 *   （同一 auth 帧格式），终端等长会话不会因 access token 轮换被踢。
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
  /** 服务端以 4003（无权限）/ 4004（对象不存在）拒绝：连接已停止重连 */
  onDenied?: () => void
}

const MAX_RETRY_MS = 30000
/** 后端关闭码：未认证 / 会话过期 */
export const WS_CLOSE_UNAUTHENTICATED = 4001
/** 后端关闭码：无权限 */
export const WS_CLOSE_FORBIDDEN = 4003
/** 后端关闭码：订阅对象不存在（重连也不会成功） */
export const WS_CLOSE_NOT_FOUND = 4004
/** 带内续期检查周期 */
export const RENEW_INTERVAL_MS = 60_000

/** 从 localStorage 读取当前 access token */
function currentToken(): string {
  return localStorage.getItem(STORAGE_KEYS.token) || ''
}

/**
 * access token 是否即将过期（< 60s）。解析 JWT payload.exp；解析失败视为「未知」→ 不主动刷新，
 * 交由 HTTP 401 链路裁决。D2 修复：此前 WS 每次 onopen 都无条件 refresh，refresh 失败即 clearAuth，
 * 导致进入终端页后整站被登出。
 */
function tokenNearExpiry(token: string, skewSec = 60): boolean {
  try {
    const payload = JSON.parse(atob(token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')))
    if (typeof payload.exp !== 'number') return false
    return payload.exp * 1000 - Date.now() < skewSec * 1000
  } catch {
    return false
  }
}

/** 仅在必要时刷新；任何失败都吞掉并回退当前 token（不在 WS 路径上清凭证） */
async function tokenForAuth(): Promise<string> {
  const cur = currentToken()
  if (!cur || !tokenNearExpiry(cur)) return cur
  try {
    return (await getRefreshedToken()) ?? currentToken()
  } catch {
    return currentToken()
  }
}

/** 生成首帧鉴权消息（R2：WS 首帧携带 token，不写入 URL） */
function authFrame(token: string): string {
  return JSON.stringify({ type: 'auth', token })
}

export function connectWithRetry(url: string, options: WSOptions): WSConnection {
  let ws: WebSocket | null = null
  let closed = false
  let retryMs = 1000
  let retryTimer: number | null = null
  let renewTimer: number | null = null
  let lastSentToken = ''

  function stopRenew() {
    if (renewTimer !== null) {
      clearInterval(renewTimer)
      renewTimer = null
    }
  }

  /** 带内续期：token 临期先刷新；与上次发送的不同才发，避免无谓帧 */
  async function renew(socket: WebSocket, gen: number) {
    const token = await tokenForAuth()
    if (gen !== currentGen || socket.readyState !== WebSocket.OPEN) return
    if (token && token !== lastSentToken) {
      socket.send(authFrame(token))
      lastSentToken = token
    }
  }
  // A2：代际计数。每次 connect 递增；旧 socket 的回调据此判失效，避免双连接。
  let currentGen = 0

  async function scheduleReconnect(authExpired = false) {
    if (closed) return
    options.onStatus?.(false)
    if (authExpired) {
      // 4001：服务端判定会话失效——强制刷新；refresh 失败时 client 已清凭证并跳转登录页
      let fresh: string | null = null
      try {
        fresh = await getRefreshedToken()
      } catch {
        fresh = null
      }
      if (!fresh) {
        closed = true
        return
      }
    } else {
      // F-33/D2: 重连前仅在 token 临期时刷新；失败不阻止重连调度，也不清凭证。
      await tokenForAuth()
    }
    // await 期间可能已被 close() 关闭，须复查
    if (closed) return
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
    // R2：WS 首帧鉴权 — 连接后首帧发送 access token，不再拼进 URL
    const socket = new WebSocket(`${protocol}//${location.host}${url}`)
    ws = socket

    socket.onopen = async () => {
      if (gen !== currentGen) return
      retryMs = 1000
      // F-33/D2: 仅在 token 临期时刷新，否则直接用当前 token 首帧鉴权
      const token = await tokenForAuth()
      // 若连接已被重建（gen 不匹配）则中止
      if (gen !== currentGen) return
      socket.send(authFrame(token))
      lastSentToken = token
      options.onStatus?.(true)
      stopRenew()
      renewTimer = window.setInterval(() => void renew(socket, gen), RENEW_INTERVAL_MS)
    }

    socket.onmessage = (ev: MessageEvent) => {
      if (gen !== currentGen) return
      try {
        options.onMessage(JSON.parse(ev.data))
      } catch {
        options.onMessage(ev.data)
      }
    }

    socket.onclose = (ev?: CloseEvent) => {
      // 旧代际 socket 关闭不再触发重连（防 reconnectNow 双连接回归）
      if (gen !== currentGen) return
      stopRenew()
      if (closed) return
      if (ev?.code === WS_CLOSE_FORBIDDEN || ev?.code === WS_CLOSE_NOT_FOUND) {
        closed = true
        options.onStatus?.(false)
        options.onDenied?.()
        return
      }
      scheduleReconnect(ev?.code === WS_CLOSE_UNAUTHENTICATED)
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
      stopRenew()
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
