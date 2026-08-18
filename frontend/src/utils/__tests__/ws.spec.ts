import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { connectWithRetry } from '@/utils/ws'

class MockWebSocket {
  static instances: MockWebSocket[] = []
  static OPEN = 1
  readyState = 0
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
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
  closeFromServer() {
    this.readyState = 3
    this.onclose?.()
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
})