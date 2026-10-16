import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import AssistantChatPanel from '@/components/aiCenter/AssistantChatPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * M-AI-3：助手对话——未启用 / 无权限空态、发送只回传文本历史、工具轨迹与审批入口、
 * 失败撤回消息并放回输入框、Enter 发送 / Shift+Enter 换行、新对话清空会话 ID。
 */

const { api, mcpApi, toast } = vi.hoisted(() => ({
  api: { chatAssistant: vi.fn() },
  mcpApi: { renderMcpPrompt: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))
vi.mock('@/api/assistant', () => api)
vi.mock('@/api/mcp', () => mcpApi)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const mounted: { unmount: () => void }[] = []

function mountPanel(props: Record<string, unknown> = {}) {
  const w = mount(AssistantChatPanel, {
    attachTo: document.body,
    props: { enabled: true, canChat: true, canConfigure: true, ...props },
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

function reply(over: Record<string, unknown> = {}) {
  return {
    data: {
      conversation_id: '7c9e6679-7425-40de-944b-e07fc1f90ae7',
      reply: '已提交审批单 #3',
      steps: [
        {
          tool: 'api_get',
          arguments: '{"path":"/api/docker/containers"}',
          ok: true,
          output: '[]',
          approval_id: null,
        },
        {
          tool: 'docker_container_restart',
          arguments: '{"container":"web"}',
          ok: true,
          output: '已提交',
          approval_id: 3,
        },
      ],
      step_limit_reached: false,
      approval_ids: [3],
      ...over,
    },
  }
}

async function type(w: ReturnType<typeof mountPanel>, text: string) {
  await w.find('textarea').setValue(text)
}

beforeEach(() => vi.clearAllMocks())
afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('AssistantChatPanel', () => {
  it('未启用：显示空态与配置提示，没有输入框', () => {
    const w = mountPanel({ enabled: false })
    expect(w.text()).toContain('助手未启用')
    expect(w.text()).toContain('助手设置')
    expect(w.find('textarea').exists()).toBe(false)
  })

  it('无 mcp:create：显示无权限空态', () => {
    const w = mountPanel({ canChat: false })
    expect(w.text()).toContain('mcp:create')
    expect(w.find('textarea').exists()).toBe(false)
  })

  it('发送：只回传 user / assistant 文本；展示工具轨迹与审批入口，并沿用会话 ID', async () => {
    api.chatAssistant.mockResolvedValueOnce(reply())
    const w = mountPanel()
    await type(w, '重启 web')
    await w.find('form').trigger('submit')
    await flush()
    expect(api.chatAssistant).toHaveBeenCalledWith([{ role: 'user', content: '重启 web' }], null)
    expect(w.text()).toContain('调用了 2 个工具')
    expect(w.text()).toContain('已生成审批单 #3')
    const go = w.findAll('button').find((b) => b.text() === '去审批')!
    await go.trigger('click')
    expect(w.emitted('open-approvals')).toHaveLength(1)

    api.chatAssistant.mockResolvedValueOnce(reply({ reply: '好的', steps: [], approval_ids: [] }))
    await type(w, '谢谢')
    await w.find('form').trigger('submit')
    await flush()
    const [history, conv] = api.chatAssistant.mock.calls[1]
    expect(history).toEqual([
      { role: 'user', content: '重启 web' },
      { role: 'assistant', content: '已提交审批单 #3' },
      { role: 'user', content: '谢谢' },
    ])
    expect(conv).toBe('7c9e6679-7425-40de-944b-e07fc1f90ae7')
  })

  it('失败：撤回用户消息并放回输入框，提示错误', async () => {
    api.chatAssistant.mockRejectedValueOnce(new Error('503'))
    const w = mountPanel()
    await type(w, '有哪些容器')
    await w.find('form').trigger('submit')
    await flush()
    expect(toast.error).toHaveBeenCalled()
    expect((w.find('textarea').element as HTMLTextAreaElement).value).toBe('有哪些容器')
    expect(w.findAll('.msg.user')).toHaveLength(0)
  })

  it('Enter 发送、Shift+Enter 不发送；新对话清空会话', async () => {
    api.chatAssistant.mockResolvedValue(reply({ steps: [], approval_ids: [] }))
    const w = mountPanel()
    await type(w, 'a')
    await w.find('textarea').trigger('keydown', { key: 'Enter', shiftKey: true })
    expect(api.chatAssistant).not.toHaveBeenCalled()
    await w.find('textarea').trigger('keydown', { key: 'Enter' })
    await flush()
    expect(api.chatAssistant).toHaveBeenCalledTimes(1)
    const reset = w.findAll('button').find((b) => b.text().includes('新对话'))!
    await reset.trigger('click')
    expect(w.findAll('.msg')).toHaveLength(0)
    await type(w, 'b')
    await w.find('form').trigger('submit')
    await flush()
    expect(api.chatAssistant.mock.calls[1][1]).toBeNull()
  })

  it('达到步数上限：提示回答可能不完整', async () => {
    api.chatAssistant.mockResolvedValueOnce(reply({ step_limit_reached: true }))
    const w = mountPanel()
    await type(w, 'x')
    await w.find('form').trigger('submit')
    await flush()
    expect(w.text()).toContain('工具调用上限')
  })

  describe('运维技能', () => {
    const runbooks = [
      { name: 'host_health_check', title: '主机健康巡检', description: '巡检', arguments: [] },
      {
        name: 'container_diagnose',
        title: '容器异常排查',
        description: '排查容器',
        arguments: [{ name: 'container', description: '容器名称或 ID', required: true }],
      },
    ]

    it('无参数技能：点击即渲染并填入输入框（不自动发送）', async () => {
      mcpApi.renderMcpPrompt.mockResolvedValueOnce({
        data: { name: 'host_health_check', title: '', text: '请巡检' },
      })
      const w = mountPanel({ runbooks })
      await w
        .findAll('button')
        .find((b) => b.text() === '主机健康巡检')!
        .trigger('click')
      await flush()
      expect(mcpApi.renderMcpPrompt).toHaveBeenCalledWith('host_health_check', {})
      expect((w.find('textarea').element as HTMLTextAreaElement).value).toBe('请巡检')
      expect(api.chatAssistant).not.toHaveBeenCalled()
    })

    it('带参数技能：必填项为空时不可提交，填写后渲染并收起表单', async () => {
      mcpApi.renderMcpPrompt.mockResolvedValueOnce({
        data: { name: 'container_diagnose', title: '', text: '排查 web' },
      })
      const w = mountPanel({ runbooks })
      await w
        .findAll('button')
        .find((b) => b.text() === '容器异常排查')!
        .trigger('click')
      expect(w.text()).toContain('容器名称或 ID *')
      const use = () => w.findAll('button').find((b) => b.text() === '填入输入框')!
      expect(use().attributes('disabled')).toBeDefined()
      await w.find('.runbook-form input').setValue('web')
      expect(use().attributes('disabled')).toBeUndefined()
      await w.find('.runbook-form').trigger('submit')
      await flush()
      expect(mcpApi.renderMcpPrompt).toHaveBeenCalledWith('container_diagnose', {
        container: 'web',
      })
      expect((w.find('textarea').element as HTMLTextAreaElement).value).toBe('排查 web')
      expect(w.find('.runbook-form').exists()).toBe(false)
    })

    it('未提供技能列表时不显示入口', () => {
      const w = mountPanel()
      expect(w.find('.runbooks').exists()).toBe(false)
    })
  })
})
