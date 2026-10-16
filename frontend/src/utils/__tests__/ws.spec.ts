import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { connectWithRetry, RENEW_INTERVAL_MS } from '@/utils/ws'

const { refreshMock } = vi.hoisted(() => ({ refreshMock: vi.fn() }))
vi.mock('@/api/client', () => ({ getRefreshedToken: refreshMock }))

class MockWebSocket {
  static instances: MockWebSocket[] = []
  static OPEN = 1
  readyState = 0
  onopen: (() => void) | null = null
  onclose: ((ev?: { code: number }) => void) | null = null
  onerror: (() => void) | null = null
  onmessage: ((ev: MessageEvent) => void) | null = null
  sent: string[] = []

  constructor(public url: string) {
    MockWebSocket.instances.push(this)
  }
  close() {
    this.readyState = 3
  }
  send(data: string) {
    this.sent.push(data)
  }
  /** 模拟服务端握手成功 */
  open() {
    this.readyState = 1
    this.onopen?.()
  }
  /** 模拟连接关闭（如服务端断开） */
  closeFromServer(code = 1006) {
    this.readyState = 3
    this.onclose?.({ code })
  }
}

describe('ws reconnectNow', () => {
  let originalLocation: Location
  let originalWebSocket: typeof WebSocket

  beforeEach(() => {
    originalWebSocket = globalThis.WebSocket
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket
    originalLocation = window.location
    Object.defineProperty(window, 'location', {
      value: { protocol: 'http:', host: 'localhost:8080' },
      writable: true,
    })
    MockWebSocket.instances = []
    vi.useFakeTimers()
  })

  afterEach(() => {
    globalThis.WebSocket = originalWebSocket
    Object.defineProperty(window, 'location', { value: originalLocation, writable: true })
    vi.useRealTimers()
  })

  it('reconnectNow 后仅保留一条活跃连接（旧 socket 关闭不触发重连）', () => {
    const conn = connectWithRetry('/ws/metrics', { onMessage: () => {} })
    expect(MockWebSocket.instances).toHaveLength(1)
    MockWebSocket.instances[0].open()

    conn.reconnectNow()
    // 立即重连：此刻已有两条 WebSocket 实例，但旧实例已 close
    expect(MockWebSocket.instances).toHaveLength(2)
    expect(MockWebSocket.instances[0].readyState).toBe(3)

    // 旧 socket 的 onclose 异步触发 —— 不得再创建第三条连接
    MockWebSocket.instances[0].closeFromServer()
    vi.runAllTimers()
    expect(MockWebSocket.instances).toHaveLength(2)

    // 新 socket 正常建立
    MockWebSocket.instances[1].open()
    expect(MockWebSocket.instances[1].readyState).toBe(1)
  })

  it('close 后旧 socket 的 onclose 不触发重连', () => {
    const conn = connectWithRetry('/ws/metrics', { onMessage: () => {} })
    MockWebSocket.instances[0].open()
    conn.close()
    MockWebSocket.instances[0].closeFromServer()
    vi.runAllTimers()
    expect(MockWebSocket.instances).toHaveLength(1)
  })

  describe('会话关闭码与带内续期', () => {
    const flush = async () => {
      for (let i = 0; i < 5; i++) await Promise.resolve()
    }

    it('4003 停止重连并通知 onDenied', async () => {
      const onDenied = vi.fn()
      const onStatus = vi.fn()
      connectWithRetry('/ws/terminal', { onMessage: () => {}, onDenied, onStatus })
      MockWebSocket.instances[0].open()
      await flush()
      MockWebSocket.instances[0].closeFromServer(4003)
      await flush()
      vi.advanceTimersByTime(60_000)
      await flush()
      expect(onDenied).toHaveBeenCalledTimes(1)
      expect(onStatus).toHaveBeenLastCalledWith(false)
      expect(MockWebSocket.instances).toHaveLength(1)
    })

    it('4004（节点不存在）同样停止重连', async () => {
      const onDenied = vi.fn()
      connectWithRetry('/ws/metrics?node_id=9', { onMessage: () => {}, onDenied })
      MockWebSocket.instances[0].open()
      await flush()
      MockWebSocket.instances[0].closeFromServer(4004)
      await flush()
      vi.advanceTimersByTime(60_000)
      await flush()
      expect(onDenied).toHaveBeenCalledTimes(1)
      expect(MockWebSocket.instances).toHaveLength(1)
    })

    it('4001 强制刷新 token 后重连；刷新失败则不再重连', async () => {
      refreshMock.mockResolvedValueOnce('new-token')
      connectWithRetry('/ws/metrics', { onMessage: () => {} })
      MockWebSocket.instances[0].open()
      await flush()
      MockWebSocket.instances[0].closeFromServer(4001)
      await flush()
      expect(refreshMock).toHaveBeenCalledTimes(1)
      vi.advanceTimersByTime(1000)
      expect(MockWebSocket.instances).toHaveLength(2)

      refreshMock.mockResolvedValueOnce(null)
      MockWebSocket.instances[1].open()
      await flush()
      MockWebSocket.instances[1].closeFromServer(4001)
      await flush()
      vi.advanceTimersByTime(60_000)
      expect(MockWebSocket.instances).toHaveLength(2)
    })

    it('token 变化后发送带内续期帧，未变化不重复发送', async () => {
      localStorage.setItem('token', 'tok-1')
      connectWithRetry('/ws/terminal', { onMessage: () => {} })
      const sock = MockWebSocket.instances[0]
      sock.open()
      await flush()
      const authFrames = () => sock.sent.filter((f) => f.includes('"type":"auth"'))
      expect(authFrames()).toHaveLength(1)
      vi.advanceTimersByTime(RENEW_INTERVAL_MS)
      await flush()
      expect(authFrames()).toHaveLength(1)
      localStorage.setItem('token', 'tok-2')
      vi.advanceTimersByTime(RENEW_INTERVAL_MS)
      await flush()
      expect(authFrames()).toHaveLength(2)
      expect(authFrames()[1]).toContain('tok-2')
      localStorage.removeItem('token')
    })
  })
})
