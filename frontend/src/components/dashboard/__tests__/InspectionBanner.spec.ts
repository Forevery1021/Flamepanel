import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import type { Ref } from 'vue'
import InspectionBanner from '@/components/dashboard/InspectionBanner.vue'
import type { Finding, InspectionReport } from '@/api/inspection'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * S-17 仪表盘安全体检横幅：只在有 critical / high 风险时出现；「本次忽略」按报告时间记忆，
 * 新报告（新的 generated_at）若仍有高危项会再次出现。
 */

const { state, push } = vi.hoisted(() => ({
  // 响应式引用在 mock 工厂里创建（hoisted 块早于 vue 的 import 执行）
  state: {} as { report: Ref<InspectionReport | null>; urgent: Ref<Finding[]> },
  push: vi.fn(),
}))

vi.mock('@/composables/useInspection', async () => {
  const { ref } = await import('vue')
  state.report = ref<InspectionReport | null>(null)
  state.urgent = ref<Finding[]>([])
  return { useInspection: () => ({ report: state.report, urgent: state.urgent }) }
})
vi.mock('vue-router', () => ({ useRouter: () => ({ push }) }))

const messages = {
  inspection: {
    banner: {
      title: '安全体检发现 {count} 个高危风险项',
      view: '查看详情',
      dismiss: '本次忽略',
    },
    severity: { critical: '严重', high: '高危', medium: '中危', low: '低危', info: '信息' },
  },
  inspectionFinding: {
    unknown: { title: '{code}', advice: '' },
    sshd_root_login: { title: 'SSH 允许 root 直接登录', advice: '' },
    exposed_service: { title: '高危服务暴露在公网', advice: '' },
  },
}

function finding(over: Partial<Finding> & Pick<Finding, 'code'>): Finding {
  return {
    category: 'access',
    severity: 'high',
    status: 'fail',
    evidence: '',
    heuristic: false,
    ...over,
  }
}

function setReport(generatedAt: string, urgent: Finding[]) {
  state.report.value = {
    generated_at: generatedAt,
    duration_ms: 1,
    cache_age_secs: 0,
    data_version: '2026-09',
    kernel: {} as InspectionReport['kernel'],
    summary: {} as InspectionReport['summary'],
    findings: urgent,
  }
  state.urgent.value = urgent
}

const mounted: { unmount: () => void }[] = []

function mountBanner() {
  const w = mount(InspectionBanner, {
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  mounted.push(w)
  return w
}

beforeEach(() => {
  sessionStorage.clear()
  push.mockReset()
  state.report.value = null
  state.urgent.value = []
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('InspectionBanner', () => {
  it('无高危项时不渲染', () => {
    setReport('2026-09-24T08:00:00Z', [])
    const w = mountBanner()
    expect(w.find('[data-testid="inspection-banner"]').exists()).toBe(false)
  })

  it('有高危项时显示计数、前 3 条预览与 +N', async () => {
    setReport('2026-09-24T08:00:00Z', [
      finding({ code: 'exposed_service', subject: '6379/tcp', severity: 'critical' }),
      finding({ code: 'sshd_root_login' }),
      finding({ code: 'exposed_service', subject: '3306/tcp' }),
      finding({ code: 'brand_new_check' }),
    ])
    const w = mountBanner()
    const banner = w.find('[data-testid="inspection-banner"]')
    expect(banner.exists()).toBe(true)
    expect(banner.text()).toContain('安全体检发现 4 个高危风险项')
    expect(banner.findAll('.banner-item')).toHaveLength(3)
    expect(banner.text()).toContain('6379/tcp')
    expect(banner.text()).toContain('+1')

    const view = banner.findAll('button').find((b) => b.text().includes('查看详情'))!
    await view.trigger('click')
    expect(push).toHaveBeenCalledWith('/inspection')
  })

  it('「本次忽略」按报告时间记忆；新报告再次出现', async () => {
    setReport('2026-09-24T08:00:00Z', [finding({ code: 'sshd_root_login' })])
    const w = mountBanner()
    const dismiss = w.findAll('button').find((b) => b.text().includes('本次忽略'))!
    await dismiss.trigger('click')
    expect(w.find('[data-testid="inspection-banner"]').exists()).toBe(false)
    expect(sessionStorage.getItem(STORAGE_KEYS.inspectionBannerDismissed)).toBe(
      '2026-09-24T08:00:00Z',
    )

    // 同一份报告：重新挂载仍保持隐藏
    const again = mountBanner()
    expect(again.find('[data-testid="inspection-banner"]').exists()).toBe(false)

    // 新一次体检仍有高危项 → 再次提示
    setReport('2026-09-24T09:00:00Z', [finding({ code: 'sshd_root_login' })])
    await again.vm.$nextTick()
    expect(again.find('[data-testid="inspection-banner"]').exists()).toBe(true)
  })
})
