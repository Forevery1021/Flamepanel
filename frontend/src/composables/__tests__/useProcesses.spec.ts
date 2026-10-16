import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import type { ProcessListParams, ProcessListResponse } from '@/api/toolbox'

/**
 * S-12 进程管理数据层（`useProcesses`）。
 *
 * ① 空参数被清洗：`{ q: '  ' }` 与 `{}` 共用一份缓存、请求不带空参；
 * ② 参数变化重新请求，期间保留上一份列表（keepPrevious）；
 * ③ kill 成功后失效 toolbox 前缀 → 列表自动重拉；
 * ④ 自动刷新开关控制轮询。
 */

const { api } = vi.hoisted(() => ({
  api: { listProcesses: vi.fn(), killProcess: vi.fn() },
}))

vi.mock('@/api/toolbox', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/toolbox')>()
  return { ...actual, ...api }
})

import { cleanParams, useProcesses, PROCESS_REFRESH_MS } from '@/composables/useProcesses'

function response(pids: number[]): ProcessListResponse {
  return {
    items: pids.map((pid) => ({
      pid,
      ppid: 1,
      name: `p${pid}`,
      user: 'root',
      uid: 0,
      command: `/bin/p${pid}`,
      cpu: 1,
      memory_bytes: 1024,
      status: 'Sleep',
      started_at: 1_700_000_000,
      kernel_thread: false,
      protected: null,
    })),
    total: pids.length,
    matched: pids.length,
    total_memory_bytes: pids.length * 1024,
    panel_pid: 1,
    sampled_at: 1_700_000_000_000,
  }
}

const mounted: { unmount: () => void }[] = []
type Ctl = ReturnType<typeof useProcesses>

function mountHost(params = ref<ProcessListParams>({}), autoRefresh = ref(false)) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  let ctl!: Ctl
  const Host = defineComponent({
    setup() {
      ctl = useProcesses(params, { autoRefresh })
      return {}
    },
    template: '<div/>',
  })
  const w = mount(Host, { global: { plugins: [[VueQueryPlugin, { queryClient }]] } })
  mounted.push(w)
  return { ctl, params, autoRefresh }
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await nextTick()
    await new Promise((r) => setTimeout(r, 5))
  }
}

beforeEach(() => {
  api.listProcesses.mockReset()
  api.killProcess.mockReset()
})

afterEach(() => {
  vi.useRealTimers()
  while (mounted.length) mounted.pop()!.unmount()
})

describe('useProcesses', () => {
  it('cleanParams 去掉空值、保留有效条件', () => {
    expect(cleanParams({ q: '   ', user: '', include_kernel: false })).toEqual({})
    expect(
      cleanParams({
        q: ' nginx ',
        user: 'root',
        sort: 'memory',
        order: 'asc',
        limit: 50,
        include_kernel: true,
      }),
    ).toEqual({
      q: 'nginx',
      user: 'root',
      sort: 'memory',
      order: 'asc',
      limit: 50,
      include_kernel: true,
    })
    expect(PROCESS_REFRESH_MS).toBe(5_000)
  })

  it('首次加载派生 rows，并以清洗后的参数请求', async () => {
    api.listProcesses.mockResolvedValue({ data: response([10, 20]) })
    const { ctl } = mountHost(ref({ q: '  ', sort: 'cpu' }))
    await settle()

    expect(api.listProcesses).toHaveBeenCalledWith({ sort: 'cpu' })
    expect(ctl.rows.value.map((r) => r.pid)).toEqual([10, 20])
    expect(ctl.data.value?.total).toBe(2)
    expect(ctl.error.value).toBeNull()
  })

  it('参数变化重新请求，新数据到达前保留旧列表', async () => {
    api.listProcesses.mockResolvedValueOnce({ data: response([10, 20]) })
    let resolveNext!: (v: { data: ProcessListResponse }) => void
    api.listProcesses.mockImplementationOnce(() => new Promise((r) => (resolveNext = r)))
    const { ctl, params } = mountHost()
    await settle()

    params.value = { q: 'nginx' }
    await settle()
    expect(api.listProcesses).toHaveBeenLastCalledWith({ q: 'nginx' })
    expect(ctl.rows.value.map((r) => r.pid)).toEqual([10, 20])

    resolveNext({ data: response([30]) })
    await settle()
    expect(ctl.rows.value.map((r) => r.pid)).toEqual([30])
  })

  it('kill 成功后失效列表并自动重拉', async () => {
    api.listProcesses.mockResolvedValueOnce({ data: response([10, 20]) })
    api.listProcesses.mockResolvedValue({ data: response([10]) })
    api.killProcess.mockResolvedValue({
      data: { pid: 20, name: 'p20', signal: 'TERM', elevated: false },
    })
    const { ctl } = mountHost()
    await settle()

    const res = await ctl.kill(20, { signal: 'TERM', started_at: 1_700_000_000 })
    await settle()

    expect(res.pid).toBe(20)
    expect(api.killProcess).toHaveBeenCalledWith(20, { signal: 'TERM', started_at: 1_700_000_000 })
    expect(api.listProcesses).toHaveBeenCalledTimes(2)
    expect(ctl.rows.value.map((r) => r.pid)).toEqual([10])
  })

  it('自动刷新开启时按间隔轮询，关闭后停止', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout'] })
    api.listProcesses.mockResolvedValue({ data: response([10]) })
    const autoRefresh = ref(true)
    mountHost(ref({}), autoRefresh)
    await vi.advanceTimersByTimeAsync(50)
    expect(api.listProcesses).toHaveBeenCalledTimes(1)

    await vi.advanceTimersByTimeAsync(PROCESS_REFRESH_MS + 50)
    expect(api.listProcesses).toHaveBeenCalledTimes(2)

    // 关闭后：已排期的那一次间隔至多再触发一次（vue-query 在下一次拉取后重新求值 refetchInterval），
    // 此后不再轮询
    autoRefresh.value = false
    await vi.advanceTimersByTimeAsync(PROCESS_REFRESH_MS + 50)
    const settled = api.listProcesses.mock.calls.length
    expect(settled).toBeLessThanOrEqual(3)
    await vi.advanceTimersByTimeAsync(PROCESS_REFRESH_MS * 3)
    expect(api.listProcesses).toHaveBeenCalledTimes(settled)
  })
})
