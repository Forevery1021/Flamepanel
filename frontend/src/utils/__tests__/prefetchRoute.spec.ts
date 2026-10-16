import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'
import { prefetchRoute, resetPrefetchState, useHoverPrefetch } from '@/utils/prefetchRoute'

/**
 * 路由级预取：只调用懒加载函数一次、悬停意图 80 ms、省流量不预取、失败可重试、
 * 已加载的组件对象与未知路径不报错。
 */

const Loaded = { name: 'Loaded', render: () => null }

function makeRouter(loader: () => Promise<unknown>) {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: Loaded },
      { path: '/docker', component: loader },
      { path: '/loaded', component: Loaded },
    ],
  })
}

function setConnection(conn: Record<string, unknown> | undefined) {
  Object.defineProperty(navigator, 'connection', { value: conn, configurable: true })
}

beforeEach(() => {
  resetPrefetchState()
  setConnection(undefined)
})
afterEach(() => {
  vi.useRealTimers()
  setConnection(undefined)
})

describe('prefetchRoute', () => {
  it('调用懒加载函数且每个路径只预取一次', () => {
    const loader = vi.fn(() => Promise.resolve({ default: Loaded }))
    const router = makeRouter(loader)
    prefetchRoute(router, '/docker')
    prefetchRoute(router, '/docker')
    expect(loader).toHaveBeenCalledTimes(1)
  })

  it('已加载组件与未匹配路径不报错', () => {
    const loader = vi.fn(() => Promise.resolve({ default: Loaded }))
    const router = makeRouter(loader)
    expect(() => prefetchRoute(router, '/loaded')).not.toThrow()
    expect(() => prefetchRoute(router, '/no-such-page')).not.toThrow()
    expect(() => prefetchRoute(router, '')).not.toThrow()
    expect(loader).not.toHaveBeenCalled()
  })

  it('加载失败后允许重试', async () => {
    const loader = vi
      .fn()
      .mockRejectedValueOnce(new Error('chunk'))
      .mockResolvedValue({ default: Loaded })
    const router = makeRouter(loader)
    prefetchRoute(router, '/docker')
    await Promise.resolve()
    await Promise.resolve()
    prefetchRoute(router, '/docker')
    expect(loader).toHaveBeenCalledTimes(2)
  })

  it('省流量模式 / 2G 网络不预取', () => {
    const loader = vi.fn(() => Promise.resolve({ default: Loaded }))
    const router = makeRouter(loader)
    setConnection({ saveData: true })
    prefetchRoute(router, '/docker')
    setConnection({ effectiveType: 'slow-2g' })
    prefetchRoute(router, '/docker')
    expect(loader).not.toHaveBeenCalled()
    setConnection({ effectiveType: '4g' })
    prefetchRoute(router, '/docker')
    expect(loader).toHaveBeenCalledTimes(1)
  })

  it('悬停意图：停留 80 ms 才预取，提前离开则取消；聚焦立即预取', () => {
    vi.useFakeTimers()
    const loader = vi.fn(() => Promise.resolve({ default: Loaded }))
    const router = makeRouter(loader)
    const h = useHoverPrefetch(router)
    h.enter('/docker')
    vi.advanceTimersByTime(50)
    h.leave()
    vi.advanceTimersByTime(200)
    expect(loader).not.toHaveBeenCalled()
    h.enter('/docker')
    vi.advanceTimersByTime(80)
    expect(loader).toHaveBeenCalledTimes(1)

    resetPrefetchState()
    h.now('/docker')
    expect(loader).toHaveBeenCalledTimes(2)
  })
})
