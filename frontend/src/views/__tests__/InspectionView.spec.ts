import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import InspectionView from '@/views/InspectionView.vue'
import type { InspectionReport } from '@/api/inspection'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * S-17 安全体检页：报告 → 评分环 / 内核卡 / 高危清单 / 分组结论；「立即体检」与「仅风险项」。
 *
 * 文案直接装载 zh-CN 语言包：页面依赖 `inspection.finding.<code>.*` 这张大表，
 * 用真实语言包既覆盖 key 存在性，也避免测试里重抄一遍。
 */

const { api } = vi.hoisted(() => ({
  api: { getInspectionReport: vi.fn(), runInspection: vi.fn() },
}))

vi.mock('@/api/inspection', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/inspection')>()
  return { ...actual, ...api }
})

function report(over: Partial<InspectionReport> = {}): InspectionReport {
  return {
    generated_at: '2026-09-24T08:00:00Z',
    duration_ms: 87,
    cache_age_secs: 125,
    data_version: '2026-09',
    kernel: {
      release: '5.4.0-150-generic',
      version: 'Linux version 5.4.0-150-generic (buildd@lcy02) #167-Ubuntu SMP',
      arch: 'x86_64',
      os_name: 'Ubuntu 20.04.6 LTS',
      os_id: 'ubuntu',
      os_version_id: '20.04',
      hostname: 'web-01',
      uptime_secs: 3 * 86_400 + 4 * 3_600,
      virtualization: 'kvm',
      reboot_required: true,
      newest_installed_kernel: '5.4.0-152-generic',
      series: '5.4',
      lts: true,
      series_eol: '2025-12-31',
    },
    summary: {
      score: 52,
      grade: 'D',
      total: 5,
      passed: 2,
      failed: 2,
      unknown: 1,
      critical: 1,
      high: 1,
      medium: 0,
      low: 0,
    },
    findings: [
      {
        code: 'kernel_eol',
        category: 'kernel',
        severity: 'medium',
        status: 'pass',
        evidence: 'vendor-maintained kernel 5.4 (ubuntu)',
        heuristic: true,
      },
      {
        code: 'sshd_root_login',
        category: 'access',
        severity: 'high',
        status: 'fail',
        evidence: 'PermitRootLogin yes',
        heuristic: false,
      },
      {
        code: 'exposed_service',
        subject: '6379/tcp',
        category: 'network',
        severity: 'critical',
        status: 'fail',
        evidence: 'redis listening on 0.0.0.0:6379',
        heuristic: false,
      },
      {
        code: 'time_sync',
        category: 'updates',
        severity: 'medium',
        status: 'unknown',
        evidence: 'timedatectl unavailable',
        heuristic: false,
      },
      {
        code: 'sysctl',
        subject: 'kernel.randomize_va_space',
        category: 'hardening',
        severity: 'high',
        status: 'pass',
        evidence: 'kernel.randomize_va_space = 2',
        heuristic: false,
      },
    ],
    ...over,
  }
}

const LayoutStub = {
  name: 'LayoutContent',
  props: { title: { type: String, default: '' }, description: { type: String, default: '' } },
  template:
    '<div class="stub-layout" :data-title="title"><slot name="actions" /><slot name="toolbar" /><slot /></div>',
}

const mounted: { unmount: () => void }[] = []

function mountView() {
  const w = mount(InspectionView, {
    global: {
      plugins: [
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { LayoutContent: LayoutStub },
    },
  })
  mounted.push(w)
  return w
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await new Promise((r) => setTimeout(r, 10))
  }
}

beforeEach(() => {
  api.getInspectionReport.mockReset()
  api.runInspection.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('InspectionView', () => {
  it('渲染评分、内核信息、高危清单与按分类分组的结论', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const w = mountView()
    await settle()

    // 评分环
    const score = w.find('[data-testid="inspection-score"]')
    expect(score.text()).toContain('52')
    expect(score.text()).toContain('等级 D')
    expect(score.classes()).toContain('is-danger')

    // 内核卡：版本 / 发行版 / 运行时长 / 待重启 / LTS
    const kernel = w.find('[data-testid="inspection-kernel"]')
    expect(kernel.text()).toContain('5.4.0-150-generic')
    expect(kernel.text()).toContain('Ubuntu 20.04.6 LTS')
    expect(kernel.text()).toContain('3d 4h')
    expect(kernel.text()).toContain('待重启')
    expect(kernel.text()).toContain('5.4.0-152-generic')

    // 高危清单：critical 在前，带 subject
    const urgent = w.findAll('[data-testid="inspection-urgent"] .urgent-item')
    expect(urgent).toHaveLength(2)
    expect(urgent[0].text()).toContain('高危服务暴露在公网')
    expect(urgent[0].text()).toContain('6379/tcp')
    expect(urgent[1].text()).toContain('SSH 允许 root 直接登录')

    // 分组顺序：kernel → hardening → access → network → updates
    const groups = w.findAll('.finding-group .panel-title').map((n) => n.text())
    expect(groups).toEqual(['内核与系统', '内核参数加固', '访问控制', '网络暴露面', '更新与时间'])

    // 风险项带建议、通过项不带建议；启发式标签只出现在 heuristic 项
    const root = w.find('.finding[data-code="sshd_root_login"]')
    expect(root.text()).toContain('PermitRootLogin yes')
    expect(root.text()).toContain('PermitRootLogin no')
    const eol = w.find('.finding[data-code="kernel_eol"]')
    expect(eol.text()).toContain('启发式')
    expect(eol.text()).not.toContain('建议')
    expect(w.text()).toContain('标注「启发式」的结论')

    // 元信息：缓存年龄 / 耗时 / 数据版本
    expect(w.text()).toContain('缓存 2 分钟前')
    expect(w.text()).toContain('耗时 87 ms')
    expect(w.text()).toContain('生命周期数据 2026-09')
  })

  it('「仅风险项」隐藏通过项与全绿分类', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const w = mountView()
    await settle()
    expect(w.findAll('.finding')).toHaveLength(5)

    const failOnly = w
      .findAll('.p-togglebutton, [role="radio"], button')
      .find((b) => b.text().includes('仅风险项'))
    expect(failOnly, '找不到「仅风险项」切换').toBeTruthy()
    await failOnly!.trigger('click')
    await settle(2)

    const codes = w.findAll('.finding').map((n) => n.attributes('data-code'))
    expect(codes.sort()).toEqual(['exposed_service', 'sshd_root_login', 'time_sync'])
    const groups = w.findAll('.finding-group .panel-title').map((n) => n.text())
    expect(groups).toEqual(['访问控制', '网络暴露面', '更新与时间'])
  })

  it('「立即体检」调用 POST 并用新报告替换页面', async () => {
    api.getInspectionReport.mockResolvedValue({ data: report() })
    api.runInspection.mockResolvedValue({
      data: report({
        generated_at: '2026-09-24T09:00:00Z',
        cache_age_secs: 0,
        summary: { ...report().summary, score: 96, grade: 'A', failed: 0, critical: 0, high: 0 },
        findings: report().findings.filter((f) => f.status !== 'fail'),
      }),
    })
    const w = mountView()
    await settle()

    await w.find('[data-testid="inspection-run"]').trigger('click')
    await settle()

    expect(api.runInspection).toHaveBeenCalledTimes(1)
    expect(api.getInspectionReport).toHaveBeenCalledTimes(1)
    expect(w.find('[data-testid="inspection-score"]').text()).toContain('96')
    expect(w.find('[data-testid="inspection-urgent"]').exists()).toBe(false)
    expect(w.text()).toContain('未发现高危风险项')
    expect(w.text()).toContain('刚刚生成')
  })

  it('加载失败显示错误态并可重试', async () => {
    api.getInspectionReport.mockRejectedValueOnce(new Error('503'))
    api.getInspectionReport.mockRejectedValueOnce(new Error('503'))
    api.getInspectionReport.mockResolvedValue({ data: report() })
    const w = mountView()
    // 数据层 retry: 1 → 两次失败（间隔约 1 s）后才进入错误态
    await new Promise((r) => setTimeout(r, 1400))
    await settle(2)

    expect(w.text()).toContain('体检报告加载失败')
    expect(w.find('[data-testid="inspection-score"]').exists()).toBe(false)

    const retry = w.findAll('button').find((b) => b.text().includes('重试'))
    expect(retry).toBeTruthy()
    await retry!.trigger('click')
    await settle(10)
    expect(w.find('[data-testid="inspection-score"]').exists()).toBe(true)
  })
})
