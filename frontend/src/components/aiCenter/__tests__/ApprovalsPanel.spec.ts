import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import ApprovalsPanel from '@/components/aiCenter/ApprovalsPanel.vue'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * M-AI-2 PR-B：审批面板——默认只看待审批 / 批准走确认且高危为 danger / 执行失败提示错误 /
 * 拒绝附理由 / 按钮可见性与后端字段级授权一致（operator 无 mcp:delete 看不到高危批准）。
 */

const { api, toast, confirmAction } = vi.hoisted(() => ({
  api: {
    listApprovals: vi.fn(),
    approveApproval: vi.fn(),
    rejectApproval: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
  confirmAction: vi.fn(),
}))

vi.mock('@/api/mcp', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction }) }))

const approval = (over: Record<string, unknown> = {}) => ({
  id: 1,
  principal_id: 5,
  session_id: null,
  tool: 'docker_container_stop',
  risk: 'write',
  arguments: '{"container":"web"}',
  preview: '停止容器 web（镜像 nginx，当前运行中）',
  status: 'pending',
  requested_at: '2026-09-28 10:00:00',
  expires_at: '2026-09-28 10:30:00',
  decided_by: null,
  decided_at: null,
  reason: '',
  result: '',
  finished_at: null,
  ...over,
})

const tools = [
  { name: 'docker_container_stop', title: '', description: '', permission: 'docker:stop', allowed: true, risk: 'write', input_schema: {} },
  { name: 'docker_container_remove', title: '', description: '', permission: 'docker:delete', allowed: true, risk: 'destructive', input_schema: {} },
]

const mounted: { unmount: () => void }[] = []

function mountPanel(role = 'admin') {
  const pinia = createPinia()
  setActivePinia(pinia)
  useAuthStore().role = role
  const w = mount(ApprovalsPanel, {
    attachTo: document.body,
    props: { principalName: (id: number) => `cred-${id}`, tools },
    global: {
      plugins: [
        pinia,
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [VueQueryPlugin, { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) }],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
    },
  })
  mounted.push(w)
  return w
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) await new Promise((r) => setTimeout(r, 10))
}

const buttons = (text: string) =>
  [...document.body.querySelectorAll('button')].filter((b) => b.textContent?.trim() === text) as HTMLButtonElement[]

beforeEach(() => {
  vi.clearAllMocks()
  api.listApprovals.mockResolvedValue({
    data: {
      data: [approval(), approval({ id: 2, tool: 'docker_container_remove', risk: 'destructive', preview: '删除容器 old' })],
      page: 1,
      page_size: 20,
      total: 2,
      total_pages: 1,
    },
  })
})

afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('ApprovalsPanel', () => {
  it('默认只拉取待审批，展示预览、凭证与风险', async () => {
    mountPanel()
    await settle()
    expect(api.listApprovals).toHaveBeenCalledWith({ status: 'pending', page: 1, page_size: 20 })
    const text = document.body.textContent ?? ''
    expect(text).toContain('停止容器 web')
    expect(text).toContain('cred-5')
    expect(text).toContain('高危')
    expect(buttons('批准')).toHaveLength(2)
  })

  it('批准走确认，高危为 danger；成功提示并刷新', async () => {
    api.approveApproval.mockResolvedValue({ data: approval({ id: 2, status: 'succeeded', result: 'ok' }) })
    mountPanel()
    await settle()
    buttons('批准')[1]!.click()
    await settle()
    const opts = confirmAction.mock.calls[0]![0] as { message: string; danger: boolean; accept: () => Promise<void> }
    expect(opts.danger).toBe(true)
    expect(opts.message).toContain('不可逆')
    expect(opts.message).toContain('删除容器 old')
    expect(api.approveApproval).not.toHaveBeenCalled()
    await opts.accept()
    expect(api.approveApproval).toHaveBeenCalledWith(2)
    expect(toast.success).toHaveBeenCalled()
  })

  it('执行失败时提示错误信息', async () => {
    api.approveApproval.mockResolvedValue({ data: approval({ status: 'failed', result: 'No such container' }) })
    mountPanel()
    await settle()
    buttons('批准')[0]!.click()
    await settle()
    const opts = confirmAction.mock.calls[0]![0] as { danger: boolean; accept: () => Promise<void> }
    expect(opts.danger).toBe(false)
    await opts.accept()
    expect(toast.error).toHaveBeenCalledWith(null, expect.stringContaining('No such container'))
  })

  it('拒绝可附理由', async () => {
    api.rejectApproval.mockResolvedValue({ data: approval({ status: 'rejected' }) })
    mountPanel()
    await settle()
    buttons('拒绝')[0]!.click()
    await settle()
    const ta = document.body.querySelector<HTMLTextAreaElement>('.fp-modal textarea')!
    ta.value = '  高峰期  '
    ta.dispatchEvent(new Event('input'))
    await settle(2)
    const confirm = [...document.body.querySelectorAll('.fp-modal__footer button')].find(
      (b) => b.textContent?.trim() === '拒绝',
    ) as HTMLButtonElement
    confirm.click()
    await settle()
    expect(api.rejectApproval).toHaveBeenCalledWith(1, '高峰期')
    expect(toast.success).toHaveBeenCalled()
  })

  it('operator 看不到高危批准；viewer 看不到任何操作', async () => {
    mountPanel('operator')
    await settle()
    expect(buttons('批准')).toHaveLength(1)
    mounted.splice(0).forEach((w) => w.unmount())
    document.body.innerHTML = ''
    mountPanel('viewer')
    await settle()
    expect(buttons('批准')).toHaveLength(0)
    expect(buttons('拒绝')).toHaveLength(0)
  })
})
