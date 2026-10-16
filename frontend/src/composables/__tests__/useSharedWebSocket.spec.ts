import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent, onMounted } from 'vue'
import { mount } from '@vue/test-utils'

// Y10：useSharedWebSocket 端点级共享连接 —— 同端点多订阅者共享一条连接，引用归零才关闭

/** mock WSConnection（保留引用以便断言 reconnectNow） */
interface MockConn {
  close: ReturnType<typeof vi.fn>
  send: ReturnType<typeof vi.fn>
  reconnectNow: ReturnType<typeof vi.fn>
}
const conns: MockConn[] = []
function makeConn(): MockConn {
  const conn: MockConn = { close: vi.fn(), send: vi.fn(() => true), reconnectNow: vi.fn() }
  conns.push(conn)
  return conn
}

const connectSpy = vi.fn()

vi.mock('@/utils/ws', () => ({
  connectWithRetry: (...args: unknown[]) => {
    const conn = makeConn()
    connectSpy(...args)
    return conn
  },
}))

import { useSharedWebSocket, isResyncFrame } from '@/composables/useSharedWebSocket'

/** 辅助组件：onMounted 时 connect（唯一的 defineComponent 调用点） */
function makeHost(url = '/ws/metrics', onMessage: (d: unknown) => void = () => {}) {
  return defineComponent({
    setup() {
      const ws = useSharedWebSocket(url, { onMessage })
      onMounted(() => ws.connect())
      return { ws }
    },
    template: '<div/>',
  })
}

describe('useSharedWebSocket 引用计数', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    conns.length = 0
  })

  it('同端点多个订阅者共享一条连接；全部卸载后才关闭', () => {
    const Host = makeHost()
    const a = mount(Host)
    const b = mount(Host)

    // 两个订阅者激活 → 只建立一条连接
    expect(connectSpy).toHaveBeenCalledTimes(1)
    expect(connectSpy.mock.calls[0][0]).toBe('/ws/metrics')

    // 卸载第一个：仍有激活订阅者，连接不关闭
    a.unmount()
    expect(connectSpy).toHaveBeenCalledTimes(1)

    // 卸载第二个：引用归零，连接关闭
    b.unmount()
    // 连接关闭本身不会再次调用 connectWithRetry
    expect(connectSpy).toHaveBeenCalledTimes(1)
  })

  it('订阅者激活数归零后，重新订阅会建立新连接', () => {
    const Host = makeHost()
    const a = mount(Host)
    const b = mount(Host)
    expect(connectSpy).toHaveBeenCalledTimes(1)

    a.unmount()
    b.unmount()

    // 重新挂载一个订阅者 → 重建连接
    const c = mount(Host)
    expect(connectSpy).toHaveBeenCalledTimes(2)
    c.unmount()
  })
})

// S-08：服务端背压控制帧 —— 收到 {type:"resync"} 立即重连，且不当作数据下发给订阅者
describe('useSharedWebSocket manageLifecycle: false（Doc/30 M-P 节点切换）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    conns.length = 0
  })

  it('不自挂卸载钩子：由调用方显式 close；可在 setup 之外（watch 回调）创建', () => {
    let ws: ReturnType<typeof useSharedWebSocket> | null = null
    const Host = defineComponent({
      setup() {
        ws = useSharedWebSocket('/ws/metrics?node_id=4', {
          onMessage: () => {},
          manageLifecycle: false,
        })
        onMounted(() => ws!.connect())
        return {}
      },
      template: '<div/>',
    })
    const w = mount(Host)
    expect(connectSpy).toHaveBeenCalledTimes(1)
    w.unmount()
    // 卸载不触发 close——连接仍在，等调用方关闭
    expect(conns[0].close).not.toHaveBeenCalled()
    ws!.close()
    expect(conns[0].close).toHaveBeenCalledTimes(1)

    // 组件实例之外创建不告警、可正常订阅
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const outside = useSharedWebSocket('/ws/metrics?node_id=5', {
      onMessage: () => {},
      manageLifecycle: false,
    })
    outside.connect()
    expect(connectSpy).toHaveBeenCalledTimes(2)
    outside.close()
    expect(warn).not.toHaveBeenCalled()
    warn.mockRestore()
  })
})

describe('useSharedWebSocket resync 控制帧', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    conns.length = 0
  })

  it('isResyncFrame 只认 type === "resync" 的对象', () => {
    expect(isResyncFrame({ type: 'resync', reason: 'lagged' })).toBe(true)
    expect(isResyncFrame({ type: 'tick', data: {} })).toBe(false)
    expect(isResyncFrame('resync')).toBe(false)
    expect(isResyncFrame(null)).toBe(false)
  })

  it('收到 resync：调用 reconnectNow，订阅者收不到该帧；普通帧照常分发', () => {
    const onMessage = vi.fn()
    const w = mount(makeHost('/ws/logs', onMessage))
    expect(connectSpy).toHaveBeenCalledTimes(1)
    const opts = connectSpy.mock.calls[0][1] as { onMessage: (d: unknown) => void }

    opts.onMessage({ type: 'tick', data: { cpu_usage: 1 } })
    expect(onMessage).toHaveBeenCalledTimes(1)

    opts.onMessage({ type: 'resync', reason: 'lagged' })
    expect(conns[0].reconnectNow).toHaveBeenCalledTimes(1)
    expect(onMessage).toHaveBeenCalledTimes(1)
    w.unmount()
  })
})
