import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import AssistantSettingsPanel from '@/components/aiCenter/AssistantSettingsPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * M-AI-3：助手设置——密钥只写不读（留空不提交 api_key）、清除密钥、先保存再测试、
 * 模型缺失提示、启用但未填模型拦截、只读角色禁用。
 */

const { api, toast } = vi.hoisted(() => ({
  api: { updateAssistantConfig: vi.fn(), testAssistant: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))
vi.mock('@/api/assistant', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const config = (over: Record<string, unknown> = {}) => ({
  enabled: true,
  provider: 'openai' as const,
  base_url: '',
  effective_base_url: 'https://api.openai.com/v1',
  model: 'gpt-4o-mini',
  has_api_key: true,
  temperature: 0.2,
  max_steps: 6,
  timeout_secs: 60,
  ...over,
})

const mounted: { unmount: () => void }[] = []

function mountPanel(props: Record<string, unknown> = {}) {
  const w = mount(AssistantSettingsPanel, {
    attachTo: document.body,
    props: { config: config(), canEdit: true, ...props },
    global: {
      plugins: [
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
      ],
      directives: { tooltip: () => {} },
    },
  })
  mounted.push(w)
  return w
}

const flush = () => new Promise((r) => setTimeout(r, 0))
const button = (w: ReturnType<typeof mountPanel>, label: string) =>
  w.findAll('button').find((b) => b.text().trim() === label)!

beforeEach(() => {
  vi.clearAllMocks()
  api.updateAssistantConfig.mockImplementation(async (p: Record<string, unknown>) => ({
    data: config({ has_api_key: p.api_key === '' ? false : true }),
  }))
})
afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('AssistantSettingsPanel', () => {
  it('保存：密钥留空时不提交 api_key（保持原值）；填写后提交并清空输入', async () => {
    const w = mountPanel()
    await button(w, '保存').trigger('click')
    await flush()
    const patch = api.updateAssistantConfig.mock.calls[0][0]
    expect(patch).not.toHaveProperty('api_key')
    expect(patch).toMatchObject({
      enabled: true,
      provider: 'openai',
      model: 'gpt-4o-mini',
      max_steps: 6,
    })
    expect(w.emitted('saved')).toHaveLength(1)

    const key = w.find('input[type="password"]')
    await key.setValue('sk-new')
    await button(w, '保存').trigger('click')
    await flush()
    expect(api.updateAssistantConfig.mock.calls[1][0].api_key).toBe('sk-new')
    expect((key.element as HTMLInputElement).value).toBe('')
  })

  it('清除已保存的 Key：只提交空字符串', async () => {
    const w = mountPanel()
    await button(w, '清除已保存的 Key').trigger('click')
    await flush()
    expect(api.updateAssistantConfig).toHaveBeenCalledWith({ api_key: '' })
  })

  it('测试连接：先保存再测试；模型不在列表中给出警告', async () => {
    api.testAssistant.mockResolvedValueOnce({
      data: { ok: true, models: ['gpt-4o'], model_found: false },
    })
    const w = mountPanel()
    await button(w, '测试连接').trigger('click')
    await flush()
    await flush()
    expect(api.updateAssistantConfig).toHaveBeenCalledTimes(1)
    expect(api.testAssistant).toHaveBeenCalledTimes(1)
    expect(toast.warning).toHaveBeenCalledWith(expect.stringContaining('gpt-4o-mini'))
  })

  it('启用但未填模型：拦截保存并提示', async () => {
    const w = mountPanel({ config: config({ model: '' }) })
    await button(w, '保存').trigger('click')
    await flush()
    expect(api.updateAssistantConfig).not.toHaveBeenCalled()
    expect(w.text()).toContain('启用助手前请填写模型名称')
  })

  it('只读角色：表单禁用、无保存 / 测试按钮', () => {
    const w = mountPanel({ canEdit: false })
    expect(w.text()).toContain('settings:update')
    expect(w.findAll('button').some((b) => b.text().trim() === '保存')).toBe(false)
    expect((w.find('input[type="password"]').element as HTMLInputElement).disabled).toBe(true)
  })
})
