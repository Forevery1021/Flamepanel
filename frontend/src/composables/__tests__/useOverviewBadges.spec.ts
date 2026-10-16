import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import type { OverviewBadges } from '@/api/overview'

/**
 * S-06 顶栏聚合徽标数据层（`useOverviewBadges`）。
 *
 * 看住三件事：
 * ① 一个响应派生出全部徽标（节点 / 容器 / 任务 / 告警明细），路由与顺序固定；
 * ② 分组为 `null`（无权限或来源不可用）时**隐藏而不是显示 0**；
 * ③ 同屏多个消费者共享同一 queryKey——只打一次请求（这是把三路轮询合并成一路的前提）。
 */

const { api } = vi.hoisted(() => ({
  api: { getOverviewBadges: vi.fn() },
}))

vi.mock('@/api/overview', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/overview')>()
  return { ...actual, ...api }
})

import { useOverviewBadges, OVERVIEW_BADGES_INTERVAL } from '@/composables/useOverviewBadges'

const full: OverviewBadges = {
  nodes: { online: 2, total: 3 },
  containers: { running: 4, total: 6 },
  tasks: { running: 1, pending: 2, failed: 3 },
  alerts: { nodes_offline: 1, certificates_expiring: 0, tasks_failed: 3, total: 4 },
  generated_at: '2026-09-24T08:00:00Z',
  cache_age_secs: 0,
}

const masked: OverviewBadges = {
  nodes: null,
  containers: null,
  tasks: null,
  alerts: null,
  generated_at: '2026-09-24T08:00:00Z',
  cache_age_secs: 3,
}

const mounted: { unmount: () => void }[] = []

type Ctl = ReturnType<typeof useOverviewBadges>

/** 挂载宿主：`count` 个消费者（各自调用 useOverviewBadges，模拟同屏多组件订阅同一 key） */
function mountHosts(count = 1) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const ctls: Ctl[] = []
  const Host = defineComponent({
    setup() {
      for (let i = 0; i < count; i++) ctls.push(useOverviewBadges())
      return {}
    },
    template: '<div/>',
  })
  const w = mount(Host, { global: { plugins: [[VueQueryPlugin, { queryClient }]] } })
  mounted.push(w)
  return ctls
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await nextTick()
    await new Promise((r) => setTimeout(r, 5))
  }
}

beforeEach(() => {
  api.getOverviewBadges.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('useOverviewBadges', () => {
  it('30 s 轮询常量（服务端 10 s TTL 缓存的 3 倍，多页签仍只有一次真实聚合）', () => {
    expect(OVERVIEW_BADGES_INTERVAL).toBe(30_000)
  })

  it('一个响应派生全部徽标：计数、进行中任务、告警明细与跳转', async () => {
    api.getOverviewBadges.mockResolvedValue({ data: full })
    const [ctl] = mountHosts()
    await settle()

    expect(ctl.panelOk.value).toBe(true)
    expect(ctl.nodes.value).toEqual({ online: 2, total: 3 })
    expect(ctl.containers.value).toEqual({ running: 4, total: 6 })
    expect(ctl.activeTasks.value).toBe(3) // running + pending
    expect(ctl.alertTotal.value).toBe(4)
    // 只列非零分项，顺序：节点 → 证书 → 任务
    expect(ctl.alertItems.value.map((a) => [a.key, a.count, a.to])).toEqual([
      ['nodesOffline', 1, '/nodes'],
      ['tasksFailed', 3, '/tasks'],
    ])
  })

  it('分组为 null（无权限 / 来源不可用）时隐藏而不是显示 0', async () => {
    api.getOverviewBadges.mockResolvedValue({ data: masked })
    const [ctl] = mountHosts()
    await settle()

    expect(ctl.panelOk.value).toBe(true)
    expect(ctl.nodes.value).toBeNull()
    expect(ctl.containers.value).toBeNull()
    expect(ctl.tasks.value).toBeNull()
    expect(ctl.activeTasks.value).toBe(0)
    expect(ctl.alertTotal.value).toBe(0)
    expect(ctl.alertItems.value).toEqual([])
  })

  it('同屏多个消费者共享一个查询：只打一次请求', async () => {
    api.getOverviewBadges.mockResolvedValue({ data: full })
    const ctls = mountHosts(3)
    await settle()

    expect(ctls).toHaveLength(3)
    expect(api.getOverviewBadges).toHaveBeenCalledTimes(1)
    for (const ctl of ctls) expect(ctl.nodes.value?.online).toBe(2)
  })

  it('请求失败：panelOk 置 false，徽标全部隐藏', async () => {
    api.getOverviewBadges.mockRejectedValue(new Error('gateway down'))
    const [ctl] = mountHosts()
    // 组合式内 retry: 1（默认 1 s 退避）——等重试结束后再断言
    await new Promise((r) => setTimeout(r, 1300))
    await settle()

    expect(ctl.panelOk.value).toBe(false)
    expect(ctl.nodes.value).toBeNull()
    expect(ctl.alertItems.value).toEqual([])
  }, 5000)
})
