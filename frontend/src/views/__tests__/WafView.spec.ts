import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import WafView from '@/views/WafView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * W3-c WAF 页：「下发到 Web 引擎」展示逐引擎报告（生效 / 未安装 / 失败 + 跳过原因 + 重部署站点），
 * 部分失败给 warning 而非 success，请求失败走 toast.error。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listWafRules: vi.fn(),
    createWafRule: vi.fn(),
    updateWafRule: vi.fn(),
    deleteWafRule: vi.fn(),
    listWafLogs: vi.fn(),
    clearWafLogs: vi.fn(),
    getWafStatus: vi.fn(),
    applyWafRules: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))

vi.mock('@/api/waf', () => api)
vi.mock('@/api/websites', () => ({
  listWebsites: vi.fn().mockResolvedValue({ data: { data: [], total: 0 } }),
}))
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const LayoutStub = {
  name: 'LayoutContent',
  template: '<div class="stub-layout"><slot name="actions" /><slot /></div>',
}

const mounted: { unmount: () => void }[] = []

function mountView() {
  const w = mount(WafView, {
    attachTo: document.body,
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
  for (let i = 0; i < times; i++) await new Promise((r) => setTimeout(r, 10))
}

function applyButton(): HTMLButtonElement {
  const btn = [...document.body.querySelectorAll('button')].find((b) =>
    b.textContent?.includes('下发到 Web 引擎'),
  )
  expect(btn, '找不到下发按钮').toBeTruthy()
  return btn as HTMLButtonElement
}

const engine = (over: Record<string, unknown>) => ({
  engine: 'nginx',
  applied: true,
  reloaded: true,
  enforced: [],
  skipped: [],
  redeployed_sites: [],
  ...over,
})

beforeEach(() => {
  for (const fn of Object.values(api)) fn.mockReset()
  for (const fn of Object.values(toast)) fn.mockReset()
  api.listWafRules.mockResolvedValue({ data: [] })
  api.listWafLogs.mockResolvedValue({
    data: { data: [], page: 1, page_size: 20, total: 0, total_pages: 0 },
  })
  api.getWafStatus.mockResolvedValue({ data: { loaded_rules: 0 } })
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
  document.body.innerHTML = ''
})

describe('WafView 下发到 Web 引擎', () => {
  it('成功：逐引擎展示状态、规则数、跳过原因与重部署站点', async () => {
    api.applyWafRules.mockResolvedValue({
      data: {
        ok: true,
        engines: [
          engine({
            enforced: [1, 2],
            skipped: [{ rule_id: 9, reason: 'CC log-only' }],
            redeployed_sites: ['a.test'],
          }),
          engine({ engine: 'openresty', applied: false, reloaded: false, reason: 'not installed' }),
        ],
      },
    })
    mountView()
    await settle()
    applyButton().click()
    await settle()
    expect(api.applyWafRules).toHaveBeenCalledTimes(1)
    const report = document.body.querySelector('.apply-report')
    expect(report).toBeTruthy()
    const text = report!.textContent ?? ''
    expect(text).toContain('nginx')
    expect(text).toContain('已生效')
    expect(text).toContain('已下发 2 条规则')
    expect(text).toContain('#9 CC log-only')
    expect(text).toContain('a.test')
    expect(text).toContain('未安装 / 无写权限')
    expect(toast.success).toHaveBeenCalled()
  })

  it('部分失败：warning 提示并展示失败原因（已回滚）', async () => {
    api.applyWafRules.mockResolvedValue({
      data: {
        ok: false,
        engines: [engine({ applied: false, reloaded: false, error: 'nginx -t: boom' })],
      },
    })
    mountView()
    await settle()
    applyButton().click()
    await settle()
    const text = document.body.querySelector('.apply-report')?.textContent ?? ''
    expect(text).toContain('下发失败（已回滚）')
    expect(text).toContain('nginx -t: boom')
    expect(toast.warning).toHaveBeenCalled()
    expect(toast.success).not.toHaveBeenCalled()
  })

  it('请求失败（如 503 未配置）走 toast.error，不显示报告', async () => {
    const err = { status: 503, code: 'SERVICE_UNAVAILABLE' }
    api.applyWafRules.mockRejectedValue(err)
    mountView()
    await settle()
    applyButton().click()
    await settle()
    expect(toast.error).toHaveBeenCalledWith(err, expect.any(String))
    expect(document.body.querySelector('.apply-report')).toBeNull()
  })
})

describe('WafView 命中日志回灌状态', () => {
  async function openLogsTab() {
    const tab = [...document.body.querySelectorAll('[role="tab"], button')].find((b) =>
      b.textContent?.includes('拦截日志'),
    ) as HTMLElement | undefined
    expect(tab, '找不到拦截日志页签').toBeTruthy()
    tab!.click()
    await settle()
  }

  it('展示各引擎导入条数与读取错误', async () => {
    api.getWafStatus.mockResolvedValue({
      data: {
        loaded_rules: 1,
        enforcement: true,
        ingest: [
          { engine: 'nginx', offset: 10, imported: 3, last_run_at: '2026-09-27T03:00:00Z' },
          { engine: 'openresty', offset: 0, imported: 0, last_error: 'helper unavailable' },
        ],
      },
    })
    mountView()
    await settle()
    await openLogsTab()
    const text = document.body.querySelector('.ingest-status')?.textContent ?? ''
    expect(text).toContain('nginx')
    expect(text).toContain('已从 Web 引擎导入 3 条')
    expect(text).toContain('日志读取失败：helper unavailable')
  })

  it('未检测到引擎时给出说明', async () => {
    api.getWafStatus.mockResolvedValue({ data: { loaded_rules: 0, enforcement: true, ingest: [] } })
    mountView()
    await settle()
    await openLogsTab()
    expect(document.body.querySelector('.ingest-status')?.textContent).toContain(
      '未检测到 Nginx / OpenResty',
    )
  })
})
