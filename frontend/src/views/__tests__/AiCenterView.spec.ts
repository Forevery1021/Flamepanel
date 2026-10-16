import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import AiCenterView from '@/views/AiCenterView.vue'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * M-AI-1 PR-B：AI 接入中心——凭证列表 / 签发后一次性展示令牌与接入片段 / 关闭即丢弃明文 /
 * 吊销走确认 / viewer 不可见签发入口。
 */

const { api, assistantApi, toast, confirmAction } = vi.hoisted(() => ({
  assistantApi: {
    getAssistantConfig: vi.fn(),
    updateAssistantConfig: vi.fn(),
    testAssistant: vi.fn(),
    chatAssistant: vi.fn(),
  },
  api: {
    listPrincipals: vi.fn(),
    createPrincipal: vi.fn(),
    revokePrincipal: vi.fn(),
    listMcpTools: vi.fn(),
    listMcpPrompts: vi.fn(),
    renderMcpPrompt: vi.fn(),
    listToolCalls: vi.fn(),
    listMcpSessions: vi.fn(),
    listApprovals: vi.fn(),
    approveApproval: vi.fn(),
    rejectApproval: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
  confirmAction: vi.fn(),
}))

vi.mock('@/api/mcp', () => api)
vi.mock('@/api/assistant', () => assistantApi)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction }) }))

const LayoutStub = {
  name: 'LayoutContent',
  template: '<div class="stub-layout"><slot name="toolbar" /><slot /></div>',
}

const TOKEN = 'fpk_' + 'b'.repeat(64)
const principal = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'cursor@laptop',
  token_prefix: 'fpk_bbbbbbbb',
  user_id: 1,
  owner: 'admin',
  active: true,
  created_at: '2026-09-01 10:00:00',
  expires_at: null,
  last_used_at: null,
  revoked_at: null,
  ...over,
})

const mounted: { unmount: () => void }[] = []

function mountView(role = 'admin', username = 'admin') {
  const pinia = createPinia()
  setActivePinia(pinia)
  useAuthStore().role = role
  useAuthStore().username = username
  const w = mount(AiCenterView, {
    attachTo: document.body,
    global: {
      plugins: [
        pinia,
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [VueQueryPlugin, { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) }],
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

function button(text: string): HTMLButtonElement | undefined {
  return [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as
    | HTMLButtonElement
    | undefined
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listPrincipals.mockResolvedValue({ data: [principal(), principal({ id: 2, name: 'old', active: false, revoked_at: '2026-09-02' })] })
  api.listMcpTools.mockResolvedValue({
    data: [{ name: 'system_status', title: '系统状态', description: 'd', permission: 'node:read', allowed: true, input_schema: {} }],
  })
  api.listToolCalls.mockResolvedValue({ data: { data: [], page: 1, page_size: 20, total: 0, total_pages: 0 } })
  api.listApprovals.mockResolvedValue({ data: { data: [], page: 1, page_size: 1, total: 2, total_pages: 2 } })
  assistantApi.getAssistantConfig.mockResolvedValue({ data: assistantConfig() })
  api.listMcpPrompts.mockResolvedValue({
    data: [
      {
        name: 'container_diagnose',
        title: '容器异常排查',
        description: '排查容器',
        arguments: [{ name: 'container', description: '容器名称或 ID', required: true }],
      },
    ],
  })
})

const assistantConfig = (over: Record<string, unknown> = {}) => ({
  enabled: false,
  provider: 'openai',
  base_url: '',
  effective_base_url: 'https://api.openai.com/v1',
  model: '',
  has_api_key: false,
  temperature: 0.2,
  max_steps: 6,
  timeout_secs: 60,
  ...over,
})

async function openAssistantTab() {
  const tab = [...document.body.querySelectorAll('.p-tab')].find((el) => el.textContent?.trim() === '助手') as
    | HTMLElement
    | undefined
  expect(tab).toBeDefined()
  tab!.click()
  await settle()
}

afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('AiCenterView', () => {
  it('列出凭证并区分有效 / 已吊销；已吊销行不提供吊销按钮', async () => {
    mountView()
    await settle()
    const text = document.body.textContent ?? ''
    expect(text).toContain('cursor@laptop')
    expect(text).toContain('有效')
    expect(text).toContain('已吊销')
    expect([...document.body.querySelectorAll('button')].filter((b) => b.textContent?.trim() === '吊销')).toHaveLength(1)
    // 列表页的接入示例只用占位令牌
    expect(text).toContain('fpk_...')
  })

  it('签发后一次性展示令牌与 Cursor 片段，「我已保存」后明文从 DOM 消失', async () => {
    api.createPrincipal.mockResolvedValue({
      data: { principal: principal({ id: 3, name: 'claude' }), token: TOKEN, endpoint: '/api/mcp' },
    })
    mountView()
    await settle()
    button('签发凭证')!.click()
    await settle()
    const input = document.body.querySelector<HTMLInputElement>('.fp-modal input')!
    input.value = 'claude'
    input.dispatchEvent(new Event('input'))
    await settle(2)
    const confirm = [...document.body.querySelectorAll('.fp-modal__footer button')].find(
      (b) => b.textContent?.trim() === '签发凭证',
    ) as HTMLButtonElement
    confirm.click()
    await settle()
    expect(api.createPrincipal).toHaveBeenCalledWith({ name: 'claude', expires_in_days: 90, allow_write: false })
    const text = document.body.textContent ?? ''
    expect(text).toContain('令牌只显示这一次')
    expect(text).toContain(TOKEN)
    expect(text).toContain(`"Authorization": "Bearer ${TOKEN}"`)
    button('我已保存')!.click()
    await settle(20)
    expect(document.body.textContent ?? '').not.toContain(TOKEN)
  })

  it('名称为空不提交', async () => {
    mountView()
    await settle()
    button('签发凭证')!.click()
    await settle()
    const confirm = [...document.body.querySelectorAll('.fp-modal__footer button')].find(
      (b) => b.textContent?.trim() === '签发凭证',
    ) as HTMLButtonElement
    confirm.click()
    await settle()
    expect(api.createPrincipal).not.toHaveBeenCalled()
    expect(document.body.textContent).toContain('请填写名称')
  })

  it('吊销需二次确认，确认后调用接口', async () => {
    api.revokePrincipal.mockResolvedValue({ data: principal({ active: false, revoked_at: 'now' }) })
    mountView()
    await settle()
    button('吊销')!.click()
    await settle()
    expect(confirmAction).toHaveBeenCalledTimes(1)
    const opts = confirmAction.mock.calls[0]![0] as { message: string; danger: boolean; accept: () => Promise<void> }
    expect(opts.message).toContain('吊销凭证「cursor@laptop」')
    expect(opts.danger).toBe(true)
    expect(api.revokePrincipal).not.toHaveBeenCalled()
    await opts.accept()
    expect(api.revokePrincipal).toHaveBeenCalledWith(1)
    expect(toast.success).toHaveBeenCalled()
  })

  it('operator 只能吊销自己的凭证（他人凭证需 mcp:delete）', async () => {
    api.listPrincipals.mockResolvedValue({
      data: [principal(), principal({ id: 3, name: 'mine', owner: 'op' })],
    })
    mountView('operator', 'op')
    await settle()
    const revokes = [...document.body.querySelectorAll('button')].filter((b) => b.textContent?.trim() === '吊销')
    expect(revokes).toHaveLength(1)
    revokes[0]!.click()
    await settle()
    const opts = confirmAction.mock.calls[0]![0] as { message: string }
    expect(opts.message).toContain('mine')
  })

  it('审批页签显示待审批数角标，凭证列表区分只读 / 可写', async () => {
    api.listPrincipals.mockResolvedValue({ data: [principal({ allow_write: true })] })
    mountView()
    await settle()
    expect(api.listApprovals).toHaveBeenCalledWith({ status: 'pending', page: 1, page_size: 1 })
    const tab = [...document.body.querySelectorAll('.p-tab')].find((el) => el.textContent?.includes('审批'))!
    expect(tab.querySelector('.fp-tabs__badge')?.textContent).toBe('2')
    expect(document.body.textContent).toContain('可写（需审批）')
  })

  it('viewer 看不到签发与吊销入口', async () => {
    mountView('viewer')
    await settle()
    expect(button('签发凭证')).toBeUndefined()
    expect(button('吊销')).toBeUndefined()
    expect(document.body.textContent).toContain('cursor@laptop')
  })

  it('助手页签（M-AI-3）：未启用时提示配置；管理员可编辑设置', async () => {
    mountView()
    await settle()
    expect(assistantApi.getAssistantConfig).toHaveBeenCalled()
    await openAssistantTab()
    const text = document.body.textContent ?? ''
    expect(text).toContain('助手未启用')
    expect(text).toContain('助手设置')
    expect(button('保存')).toBeDefined()
    expect(document.body.querySelector('textarea')).toBeNull()
  })

  it('助手页签：viewer 无对话权限，设置只读', async () => {
    assistantApi.getAssistantConfig.mockResolvedValue({
      data: assistantConfig({ enabled: true, provider: 'ollama', model: 'qwen2.5' }),
    })
    mountView('viewer')
    await settle()
    await openAssistantTab()
    const text = document.body.textContent ?? ''
    expect(text).toContain('mcp:create')
    expect(text).toContain('settings:update')
    expect(button('保存')).toBeUndefined()
  })

  it('工具目录页签展示运维技能（Prompts）及参数', async () => {
    mountView()
    await settle()
    const tab = [...document.body.querySelectorAll('.p-tab')].find((el) => el.textContent?.trim() === '工具目录') as
      | HTMLElement
      | undefined
    expect(tab).toBeDefined()
    tab!.click()
    await settle()
    const text = document.body.textContent ?? ''
    expect(api.listMcpPrompts).toHaveBeenCalled()
    expect(text).toContain('运维技能（Prompts）')
    expect(text).toContain('container_diagnose')
    expect(text).toContain('container*')
  })
})
