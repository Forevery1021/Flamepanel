import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import api, { attachAuthInterceptor, attachErrorInterceptor, longApi } from '@/api/client'
import axios, { AxiosError, AxiosHeaders } from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { STORAGE_KEYS } from '@/utils/storage'
import { useAuthStore } from '@/stores/auth'

// Y10：401 单飞去重 —— 并发多个 401 时，refreshToken 只应被调用一次
const refreshSpy = vi.fn()

vi.mock('@/api/auth', () => ({
  refreshToken: (...args: unknown[]) => refreshSpy(...args),
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

/**
 * 自定义 axios adapter：模拟后端响应。
 * 注意：axios 的 dispatchRequest 对自定义 adapter 不做状态码校验（不调用 settle），
 * 因此 adapter 需对非 2xx 主动 reject AxiosError，才能触发响应拦截器的错误分支。
 */
function installAdapter(
  handler: (config: { url?: string; auth?: string }) => { status: number; data: unknown },
) {
  api.defaults.adapter = async (config) => {
    const url = (config.url ?? '').replace('/api', '')
    const auth = (config.headers?.Authorization as string | undefined) ?? ''
    const res = handler({ url, auth })
    if (res.status >= 400) {
      throw new AxiosError(
        `Request failed with status code ${res.status}`,
        AxiosError.ERR_BAD_REQUEST,
        config,
        undefined,
        {
          data: res.data,
          status: res.status,
          statusText: String(res.status),
          headers: new AxiosHeaders(),
          config,
        } as never,
      )
    }
    return {
      data: res.data,
      status: res.status,
      statusText: String(res.status),
      headers: new AxiosHeaders(),
      config,
    }
  }
}

describe('client 401 → refresh 单飞去重', () => {
  const originalAdapter = api.defaults.adapter

  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem(STORAGE_KEYS.token, 'old-token')
    // 默认刷新成功
    refreshSpy.mockResolvedValue({
      data: { token: 'new-token', username: 'admin', role: 'admin' },
    })
  })

  afterEach(() => {
    api.defaults.adapter = originalAdapter
    vi.clearAllMocks()
  })

  it('并发多个业务请求同时 401 时，refreshToken 只调用一次', async () => {
    installAdapter(({ auth }) => {
      // 带旧 token 的首次请求 → 401；带新 token 的重放 → 200
      if (auth === 'Bearer new-token') {
        return { status: 200, data: { ok: true } }
      }
      return { status: 401, data: { error: 'unauthorized', message: 'expired' } }
    })

    const [a, b, c] = await Promise.all([api.get('/users'), api.get('/nodes'), api.get('/logs')])

    // 三个请求最终都成功（拿到新 token 后重放）
    expect(a.status).toBe(200)
    expect(b.status).toBe(200)
    expect(c.status).toBe(200)
    // 关键：并发 401 只触发一次 refresh（单飞去重）
    expect(refreshSpy).toHaveBeenCalledTimes(1)
    // 新 token 已写入 localStorage
    expect(localStorage.getItem(STORAGE_KEYS.token)).toBe('new-token')
  })

  it('刷新失败时清理本地凭据并跳转登录页', async () => {
    // 刷新失败
    refreshSpy.mockRejectedValue(new Error('refresh failed'))
    installAdapter(() => ({ status: 401, data: { error: 'unauthorized', message: 'expired' } }))

    // 阻止 jsdom 真正跳转，改为记录目标
    const originalLocation = window.location
    let navigatedTo = ''
    const fakeLocation = {
      href: '/dashboard',
      pathname: '/dashboard',
      search: '',
      hash: '',
      origin: 'http://localhost',
      protocol: 'http:',
      host: 'localhost',
      hostname: 'localhost',
      port: '',
      assign: vi.fn(),
      replace: vi.fn(),
      reload: vi.fn(),
      toString: () => '/dashboard',
      ancestorOrigins: [] as string[],
    } as unknown as Location
    Object.defineProperty(fakeLocation, 'href', {
      get: () => fakeLocation.pathname,
      set: (v: string) => {
        navigatedTo = v
      },
      configurable: true,
    })
    Object.defineProperty(window, 'location', {
      value: fakeLocation,
      writable: true,
      configurable: true,
    })

    await expect(api.get('/users')).rejects.toThrow()

    expect(localStorage.getItem(STORAGE_KEYS.token)).toBeNull()
    expect(navigatedTo).toBe('/login')

    Object.defineProperty(window, 'location', {
      value: originalLocation,
      writable: true,
      configurable: true,
    })
  })
})

/**
 * T0-2（Doc/23 §3.1 / P0-3、P1-5）：401 刷新链路必须把新角色**同步给 auth store**。
 *
 * 本模块被 store 反向依赖（store → api/auth → client），不能 import store（成环），
 * 只能在刷新成功后直写存储。而 `useStorage` 只监听原生 `storage` 事件——该事件仅在
 * **其它文档**触发，同文档直写不会通知订阅者，于是「存储已是新值、store 仍是旧值」，
 * `v-permission` 永远看不到刷新后的角色。修复：回写统一走 `writeStorage`（写盘 + 广播）。
 */
describe('client 401 刷新 → 角色同步到 auth store（T0-2 单一来源）', () => {
  const originalAdapter = api.defaults.adapter

  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    localStorage.setItem(STORAGE_KEYS.token, 'old-token')
    // 刷新接口返回「服务端下发的角色」——与本地存储中的旧角色不同
    refreshSpy.mockResolvedValue({
      data: { token: 'new-token', username: 'admin', role: 'admin' },
    })
  })

  afterEach(() => {
    api.defaults.adapter = originalAdapter
    vi.clearAllMocks()
  })

  it('【关键回归】刷新拿到新角色后，store 的 role 实时更新', async () => {
    const auth = useAuthStore()
    auth.role = 'viewer'
    await nextTick()

    installAdapter(({ auth: authHeader }) =>
      authHeader === 'Bearer new-token'
        ? { status: 200, data: { ok: true } }
        : { status: 401, data: { error: 'unauthorized', message: 'expired' } },
    )

    await api.get('/users')
    await nextTick()

    // 单一来源已同步（v-permission 等订阅方据此重算）
    expect(auth.role).toBe('admin')
    expect(localStorage.getItem(STORAGE_KEYS.role)).toBe('admin')
  })

  it('刷新失败清空凭证时，store 不再残留旧角色（角色被清空）', async () => {
    const auth = useAuthStore()
    auth.role = 'admin'
    await nextTick()
    refreshSpy.mockRejectedValue(new Error('refresh failed'))
    installAdapter(() => ({ status: 401, data: { error: 'unauthorized', message: 'expired' } }))

    // 阻止 jsdom 真正跳转
    const originalLocation = window.location
    Object.defineProperty(window, 'location', {
      value: { pathname: '/login', href: '/login' },
      writable: true,
      configurable: true,
    })
    try {
      await expect(api.get('/users')).rejects.toThrow()
      await nextTick()
      expect(auth.token).toBe('')
      expect(auth.role).toBe('')
    } finally {
      Object.defineProperty(window, 'location', {
        value: originalLocation,
        writable: true,
        configurable: true,
      })
    }
  })
})

/**
 * T2-1（Doc/23 §3.3 / P1-2）：`api` 与 `longApi` 的拦截器**单源化**。
 *
 * 既有缺口（P1-2）：两个实例各维护一份**逐行相同**的请求/响应拦截器。双份维护下
 * 「改一处漏一处」只是时间问题——`longApi` 那份已经漏掉了 api 那份的「刷新失败已跳
 * 登录」注释与 `clearAuth` 时序，正是分化的前兆。
 *
 * 修复后二者由同一对工厂（`attachAuthInterceptor` / `attachErrorInterceptor`）装配，
 * 行为由同一份实现决定。本组用例锁四件事：
 *
 * 1. **工厂可独立装配**——任意新实例调用工厂后即刻获得鉴权头注入、401 重放与错误规范化；
 * 2. **401 重放归属正确**——`longApi` 必须重放回 `longApi` 自身。若被误写成重放回 `api`，
 *    长操作（安装/备份/恢复，timeout: 0）会退回 15s 默认超时，属于 F-33 回归；
 * 3. **错误规范化行为一致**——同一后端载荷在两个实例上产出同一前端错误对象；
 * 4. **拦截器不重复注册**——工厂一旦被调用两次，Authorization 头会重复注入。
 */
describe('client 实例装配（T2-1 拦截器单源化）', () => {
  const originalApiAdapter = api.defaults.adapter
  const originalLongAdapter = longApi.defaults.adapter

  /**
   * 为任意实例安装记录型 adapter（记录调用与鉴权头，按处理器决定响应）。
   *
   * 注意：adapter 必须对非 2xx **主动 reject**——axios 的 dispatchRequest 对自定义
   * adapter 不做状态码校验（不调用 settle），返回 4xx 响应体不会被当成错误。
   */
  function installAdapterOn(
    instance: typeof api,
    calls: Array<{ url: string; auth: string }>,
    handler: (call: { url: string; auth: string }) => { status: number; data: unknown },
  ) {
    instance.defaults.adapter = async (config) => {
      const call = {
        url: (config.url ?? '').replace('/api', ''),
        auth: (config.headers?.Authorization as string | undefined) ?? '',
      }
      calls.push(call)
      const res = handler(call)
      if (res.status >= 400) {
        throw new AxiosError(
          `Request failed with status code ${res.status}`,
          AxiosError.ERR_BAD_REQUEST,
          config,
          undefined,
          {
            data: res.data,
            status: res.status,
            statusText: String(res.status),
            headers: new AxiosHeaders(),
            config,
          } as never,
        )
      }
      return {
        data: res.data,
        status: res.status,
        statusText: String(res.status),
        headers: new AxiosHeaders(),
        config,
      }
    }
  }

  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem(STORAGE_KEYS.token, 'old-token')
    refreshSpy.mockResolvedValue({
      data: { token: 'new-token', username: 'admin', role: 'admin' },
    })
    // 阻止 jsdom 真正跳转（刷新失败路径会赋值 location.href）
    Object.defineProperty(window, 'location', {
      value: { pathname: '/dashboard', href: '/dashboard' },
      writable: true,
      configurable: true,
    })
  })

  afterEach(() => {
    api.defaults.adapter = originalApiAdapter
    longApi.defaults.adapter = originalLongAdapter
    vi.clearAllMocks()
  })

  // ── 装配结果：两个实例都拿到了同一对拦截器 ──────────────────
  it('api 与 longApi 均装配了恰好一个请求拦截器与一个响应拦截器（不重复注册）', () => {
    // 重复 attach 会让 Authorization 头被注入两次（同 key 覆盖虽无害，但属装配错误），
    // 故这里把「装配数量」也锁进契约。
    expect(api.interceptors.request.handlers).toHaveLength(1)
    expect(api.interceptors.response.handlers).toHaveLength(1)
    expect(longApi.interceptors.request.handlers).toHaveLength(1)
    expect(longApi.interceptors.response.handlers).toHaveLength(1)
  })

  it('两个实例都会注入 Authorization 头（含 longApi 长操作实例）', async () => {
    const apiCalls: Array<{ url: string; auth: string }> = []
    const longCalls: Array<{ url: string; auth: string }> = []
    installAdapterOn(api, apiCalls, () => ({ status: 200, data: { ok: true } }))
    installAdapterOn(longApi, longCalls, () => ({ status: 200, data: { ok: true } }))

    await api.get('/users')
    await longApi.post('/backups/x/restore')

    expect(apiCalls).toEqual([{ url: '/users', auth: 'Bearer old-token' }])
    expect(longCalls).toEqual([{ url: '/backups/x/restore', auth: 'Bearer old-token' }])
  })

  // ── 关键回归：重放归属 ──────────────────────────────────────
  it('【关键回归】longApi 的 401 重放不借用 api 的默认配置（归 longApi 自身）', async () => {
    // 观测手段：只给 api 的 defaults 打一个独有标记头。
    // axios 重放走 `instance(config)` → `mergeConfig(instance.defaults, config)`，
    // 故若重放目标被误写成 `api(config)`（旧行为的最可能回归形态），
    // api 的标记头必然出现在重放请求上；归 longApi 时则从头到尾都不该出现。
    //
    // 为什么不用「换个 adapter 看谁收到」：config.adapter 在 mergeConfig 中由**请求
    // config 一方优先**，重放会沿用原实例的 adapter，观察不到归属差异（踩过）。
    const MARKER = 'X-Test-Replay-Owner'
    api.defaults.headers.common[MARKER] = 'api'
    try {
      const longCalls: Array<{ url: string; auth: string; marker: string }> = []
      const apiCalls: Array<{ url: string; auth: string; marker: string }> = []
      const record = (calls: typeof longCalls) => (cfg: { url?: string; headers?: unknown }) => {
        const h = cfg.headers as Record<string, unknown> | undefined
        calls.push({
          url: (cfg.url ?? '').replace('/api', ''),
          auth: String(h?.Authorization ?? ''),
          marker: String(h?.[MARKER] ?? ''),
        })
      }
      api.defaults.adapter = async (config) => {
        record(apiCalls)(config)
        return {
          data: { ok: true },
          status: 200,
          statusText: '200',
          headers: new AxiosHeaders(),
          config,
        }
      }
      longApi.defaults.adapter = async (config) => {
        record(longCalls)(config)
        if (config.headers?.Authorization === 'Bearer new-token') {
          return {
            data: { ok: true },
            status: 200,
            statusText: '200',
            headers: new AxiosHeaders(),
            config,
          }
        }
        throw new AxiosError(
          'Request failed with status code 401',
          AxiosError.ERR_BAD_REQUEST,
          config,
          undefined,
          {
            data: { error: 'unauthorized', message: 'expired' },
            status: 401,
            statusText: '401',
            headers: new AxiosHeaders(),
            config,
          } as never,
        )
      }

      const res = await longApi.post('/backups/x/restore')

      expect(res.status).toBe(200)
      expect(refreshSpy).toHaveBeenCalledTimes(1)
      // 重放请求（第 2 条）既带新 token，也不该带上 api 的标记头
      expect(longCalls).toEqual([
        { url: '/backups/x/restore', auth: 'Bearer old-token', marker: '' },
        { url: '/backups/x/restore', auth: 'Bearer new-token', marker: '' },
      ])
      // api 实例整场未被借用
      expect(apiCalls).toHaveLength(0)
    } finally {
      delete api.defaults.headers.common[MARKER]
    }
  })

  it('longApi 只重放一次（_retried 幂等），刷新失败时不再重试', async () => {
    const longCalls: Array<{ url: string; auth: string }> = []
    refreshSpy.mockResolvedValue({ data: { token: 'still-bad', username: 'admin', role: 'admin' } })
    installAdapterOn(longApi, longCalls, () => ({
      status: 401,
      data: { error: 'unauthorized', message: 'expired' },
    }))

    await expect(longApi.post('/backups/x/restore')).rejects.toMatchObject({ status: 401 })

    // 首次 + 重放一次，共 2 次；不会因仍然 401 而无限重试
    expect(longCalls).toHaveLength(2)
    expect(refreshSpy).toHaveBeenCalledTimes(1)
  })

  // ── 行为单源：两个实例的规范化结果逐字段一致 ────────────────
  it('同一后端错误载荷在两个实例上规范化出同一前端错误对象', async () => {
    const payload = {
      error: 'VALIDATION_ERROR',
      message: '域名格式不正确',
    }
    const apiCalls: Array<{ url: string; auth: string }> = []
    const longCalls: Array<{ url: string; auth: string }> = []
    installAdapterOn(api, apiCalls, () => ({ status: 400, data: payload }))
    installAdapterOn(longApi, longCalls, () => ({ status: 400, data: payload }))

    const fromApi = await api.get('/websites').catch((e) => e as never)
    const fromLong = await longApi.post('/backups/x/restore').catch((e) => e as never)

    expect(fromApi).toMatchObject({
      code: 'VALIDATION_ERROR',
      message: '域名格式不正确',
      status: 400,
    })
    expect(fromLong).toMatchObject({
      code: 'VALIDATION_ERROR',
      message: '域名格式不正确',
      status: 400,
    })
  })

  // ── 工厂可独立装配（单源化后的复用面） ──────────────────────
  it('工厂可装配到任意新实例：新实例即刻具备鉴权头注入、401 重放与错误规范化', async () => {
    const fresh = axios.create({ baseURL: '/api', timeout: 0, withCredentials: true })
    attachAuthInterceptor(fresh)
    attachErrorInterceptor(fresh)

    const calls: Array<{ url: string; auth: string }> = []
    installAdapterOn(fresh as unknown as typeof api, calls, ({ url, auth }) => {
      if (url === '/fresh/replay' && auth === 'Bearer new-token') {
        return { status: 200, data: { ok: true } }
      }
      if (url === '/fresh/replay') {
        return { status: 401, data: { error: 'unauthorized', message: 'expired' } }
      }
      return { status: 422, data: { error: 'UNPROCESSABLE', message: 'bad payload' } }
    })

    await expect(fresh.post('/fresh/replay')).resolves.toMatchObject({ status: 200 })
    expect(calls[0]).toEqual({ url: '/fresh/replay', auth: 'Bearer old-token' })

    await expect(fresh.post('/fresh/other')).rejects.toMatchObject({
      code: 'UNPROCESSABLE',
      status: 422,
    })
  })
})
