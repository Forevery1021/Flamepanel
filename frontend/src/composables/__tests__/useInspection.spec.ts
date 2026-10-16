import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import type { InspectionReport } from '@/api/inspection'

/**
 * S-17 体检数据层（`useInspection`）。
 *
 * ① 同屏多个消费者（页面 + 仪表盘横幅）共享一个查询：只打一次 GET；
 * ② `run()` 成功后直接把新报告写入缓存，不再多一次 GET；
 * ③ `enabled = false` 时不发请求（无 `toolbox:read` 的角色）；
 * ④ 派生值：分组按分类顺序、`urgent` 只含高危 fail。
 */

const { api } = vi.hoisted(() => ({
  api: { getInspectionReport: vi.fn(), runInspection: vi.fn() },
}))

vi.mock('@/api/inspection', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/inspection')>()
  return { ...actual, ...api }
})

import { useInspection, INSPECTION_REFETCH_MS } from '@/composables/useInspection'

function report(over: Partial<InspectionReport> = {}): InspectionReport {
  return {
    generated_at: '2026-09-24T08:00:00Z',
    duration_ms: 120,
    cache_age_secs: 0,
    data_version: '2026-09',
    kernel: {
      release: '6.1.0-25-amd64',
      version: 'Linux version 6.1.0-25-amd64',
      arch: 'x86_64',
      os_name: 'Debian GNU/Linux 12 (bookworm)',
      os_id: 'debian',
      os_version_id: '12',
      hostname: 'srv',
      uptime_secs: 3600,
      reboot_required: false,
    },
    summary: {
      score: 70,
      grade: 'C',
      total: 3,
      passed: 1,
      failed: 2,
      unknown: 0,
      critical: 1,
      high: 0,
      medium: 1,
      low: 0,
    },
    findings: [
      {
        code: 'time_sync',
        category: 'updates',
        severity: 'medium',
        status: 'fail',
        evidence: 'NTPSynchronized=no',
        heuristic: false,
      },
      {
        code: 'uid0_accounts',
        category: 'access',
        severity: 'critical',
        status: 'fail',
        evidence: 'toor',
        heuristic: false,
      },
      {
        code: 'kernel_eol',
        category: 'kernel',
        severity: 'medium',
        status: 'pass',
        evidence: '6.1 LTS until 2033-12-31',
        heuristic: true,
      },
    ],
    ...over,
  }
}

const mounted: { unmount: () => void }[] = []
type Ctl = ReturnType<typeof useInspection>

function mountHosts(count = 1, options?: Parameters<typeof useInspection>[0]) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const ctls: Ctl[] = []
  const Host = defineComponent({
    setup() {
      for (let i = 0; i < count; i++) ctls.push(useInspection(options))
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
  api.getInspectionReport.mockReset()
  api.runInspection.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('useInspection', () => {
  it('后台补刷间隔与服务端 10 min 缓存同步', () => {
    expect(INSPECTION_REFETCH_MS).toBe(600_000)
  })

  it('一个响应派生报告 / 分组 / 高危项', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const [ctl] = mountHosts()
    await settle()

    expect(ctl.report.value?.kernel.release).toBe('6.1.0-25-amd64')
    expect(ctl.groups.value.map((g) => g.category)).toEqual(['kernel', 'access', 'updates'])
    expect(ctl.urgent.value.map((x) => x.code)).toEqual(['uid0_accounts'])
    expect(ctl.error.value).toBeNull()
  })

  it('同屏多个消费者共享一个查询：只打一次请求', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const ctls = mountHosts(3)
    await settle()

    expect(api.getInspectionReport).toHaveBeenCalledTimes(1)
    for (const ctl of ctls) expect(ctl.report.value?.summary.score).toBe(70)
  })

  it('run() 成功后新报告直接进缓存，所有消费者同步且不再多一次 GET', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const fresh = report({
      generated_at: '2026-09-24T09:00:00Z',
      summary: { ...report().summary, score: 95, grade: 'A', failed: 0, critical: 0 },
      findings: [],
    })
    api.runInspection.mockResolvedValue({ data: fresh })
    const [a, b] = mountHosts(2)
    await settle()

    const result = await a.run()
    await settle()

    expect(result.summary.score).toBe(95)
    expect(api.runInspection).toHaveBeenCalledTimes(1)
    expect(api.getInspectionReport).toHaveBeenCalledTimes(1)
    expect(b.report.value?.generated_at).toBe('2026-09-24T09:00:00Z')
    expect(b.urgent.value).toEqual([])
  })

  it('run() 失败暴露 runError，原报告保持', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    api.runInspection.mockRejectedValue(new Error('boom'))
    const [ctl] = mountHosts()
    await settle()

    await expect(ctl.run()).rejects.toThrow('boom')
    await settle()
    expect(ctl.runError.value).toBeTruthy()
    expect(ctl.report.value?.summary.score).toBe(70)
  })

  it('enabled = false 时不发请求', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const [ctl] = mountHosts(1, { enabled: false })
    await settle()

    expect(api.getInspectionReport).not.toHaveBeenCalled()
    expect(ctl.report.value).toBeNull()
    expect(ctl.groups.value).toEqual([])
  })

  it('请求失败 → 只重试一次后 error 非空、report 为 null', async () => {
    api.getInspectionReport.mockRejectedValue(new Error('network down'))
    const [ctl] = mountHosts()
    // retry: 1 → 首次失败后约 1 s 重试一次，再失败才进入错误态
    await new Promise((r) => setTimeout(r, 1400))
    await settle(2)

    expect(api.getInspectionReport).toHaveBeenCalledTimes(2)
    expect(ctl.report.value).toBeNull()
    expect(ctl.error.value).toBeTruthy()
  })
})
