import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import TerminalView from '@/views/TerminalView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * Web 终端（本机 / 节点远程终端 Agent Edge）。
 *
 * xterm 与 WS 层均替身：断言的是**协议接线**——端点随 `nodeId` 切换、output 写入终端、
 * exit / error 帧的处理（不可恢复错误停止自动重连，EDGE_OFFLINE 交给退避重连），
 * 以及连接建立 / 尺寸变化时同步 resize。
 */

const h = vi.hoisted(() => ({
  url: '',
  opts: null as null | {
    onStatus?: (c: boolean) => void
    onMessage: (d: unknown) => void
  },
  ws: {
    connect: vi.fn(),
    close: vi.fn(),
    send: vi.fn(() => true),
    reconnectNow: vi.fn(),
  },
  term: {
    written: [] as string[],
    cols: 120,
    rows: 30,
    onResizeCb: null as null | (() => void),
  },
}))

vi.mock('@/composables/useWebSocket', () => ({
  useWebSocket: (url: string, opts: typeof h.opts) => {
    h.url = url
    h.opts = opts
    return h.ws
  },
}))

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    cols = h.term.cols
    rows = h.term.rows
    loadAddon() {}
    open() {}
    focus() {}
    clear() {}
    dispose() {}
    onData() {}
    onResize(cb: () => void) {
      h.term.onResizeCb = cb
    }
    write(s: string) {
      h.term.written.push(s)
    }
  },
}))
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {}
  },
}))
vi.mock('@xterm/xterm/css/xterm.css', () => ({}))

const messages = {
  terminal: {
    title: 'Web 终端',
    connected: '已连接',
    disconnected: '未连接',
    reconnect: '重连',
    clear: '清屏',
    ctrlC: 'Ctrl+C',
    ctrlD: 'Ctrl+D',
    remoteTitle: '远程终端 · {name}',
    exited: '会话已结束（退出码 {code}）',
    errTERMINAL_DISABLED: '未开启远程终端',
    errTOO_MANY_TERMINALS: '终端已达上限',
    errEDGE_OFFLINE: 'Edge 未在线',
  },
}

function mountView(props: Record<string, unknown> = {}) {
  return mount(TerminalView, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
    },
  })
}

const written = () => h.term.written.join('')

beforeEach(() => {
  h.url = ''
  h.opts = null
  h.term.written = []
  h.term.onResizeCb = null
  Object.values(h.ws).forEach((f) => f.mockClear())
})

describe('TerminalView', () => {
  it('本机终端：端点 /ws/terminal，挂载即连接', () => {
    const w = mountView()
    expect(h.url).toBe('/ws/terminal')
    expect(h.ws.connect).toHaveBeenCalled()
    expect(w.text()).toContain('Web 终端')
  })

  it('【关键】节点远程终端：端点带 node_id，标题带节点名', () => {
    const w = mountView({ nodeId: 7, nodeName: 'edge-1' })
    expect(h.url).toBe('/ws/terminal?node_id=7')
    expect(w.text()).toContain('远程终端 · edge-1')
  })

  it('output 帧写入终端', () => {
    mountView({ nodeId: 7 })
    h.opts?.onMessage({ type: 'output', data: 'hello' })
    expect(written()).toContain('hello')
  })

  it('【关键】连接建立与尺寸变化时同步 resize 给 PTY', () => {
    mountView({ nodeId: 7 })
    h.opts?.onStatus?.(true)
    expect(h.ws.send).toHaveBeenCalledWith(JSON.stringify({ type: 'resize', cols: 120, rows: 30 }))
    h.ws.send.mockClear()
    h.term.onResizeCb?.()
    expect(h.ws.send).toHaveBeenCalledWith(JSON.stringify({ type: 'resize', cols: 120, rows: 30 }))
  })

  it('【关键】TERMINAL_DISABLED：提示原因并停止自动重连；点「重连」重新订阅而非 reconnectNow', async () => {
    const w = mountView({ nodeId: 7 })
    h.opts?.onMessage({ type: 'error', code: 'TERMINAL_DISABLED', message: 'x' })
    expect(written()).toContain('未开启远程终端')
    expect(h.ws.close).toHaveBeenCalledTimes(1)

    h.ws.connect.mockClear()
    const btn = w.findAll('button').find((b) => b.text().includes('重连'))
    await btn?.trigger('click')
    expect(h.ws.connect).toHaveBeenCalledTimes(1)
    expect(h.ws.reconnectNow).not.toHaveBeenCalled()
  })

  it('【关键】EDGE_OFFLINE：只提示，不停止（交给退避重连等待 Agent 上线）', async () => {
    const w = mountView({ nodeId: 7 })
    h.opts?.onMessage({ type: 'error', code: 'EDGE_OFFLINE', message: 'x' })
    expect(written()).toContain('Edge 未在线')
    expect(h.ws.close).not.toHaveBeenCalled()
    const btn = w.findAll('button').find((b) => b.text().includes('重连'))
    await btn?.trigger('click')
    expect(h.ws.reconnectNow).toHaveBeenCalledTimes(1)
  })

  it('未知错误码回落服务端 message', () => {
    mountView({ nodeId: 7 })
    h.opts?.onMessage({ type: 'error', code: 'WEIRD', message: 'boom' })
    expect(written()).toContain('boom')
    expect(h.ws.close).toHaveBeenCalled()
  })

  it('【关键】exit 帧：提示退出码并结束会话', () => {
    mountView({ nodeId: 7 })
    h.opts?.onMessage({ type: 'exit', code: 0 })
    expect(written()).toContain('会话已结束（退出码 0）')
    expect(h.ws.close).toHaveBeenCalled()
  })
})
