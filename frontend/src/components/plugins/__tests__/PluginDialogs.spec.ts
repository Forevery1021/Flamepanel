import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import PluginExecuteDialog from '@/components/plugins/PluginExecuteDialog.vue'
import PluginCapabilitiesDialog from '@/components/plugins/PluginCapabilitiesDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/** batch-14：执行对话框（input → output/logs/耗时）与能力授予对话框 */

const { api, toast } = vi.hoisted(() => ({
  api: { executePlugin: vi.fn(), updatePluginCapabilities: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))
vi.mock('@/api/plugins', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const mounted: { unmount: () => void }[] = []
function mountOptions() {
  return {
    attachTo: document.body,
    global: {
      plugins: [
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
      ],
      directives: { permission: () => {}, tooltip: () => {} },
    },
  }
}
function mountExecute(pluginId: string) {
  const w = mount(PluginExecuteDialog, {
    ...mountOptions(),
    props: { modelValue: false, pluginId },
  })
  mounted.push(w)
  return w
}
function mountCapabilities(
  plugin: InstanceType<typeof PluginCapabilitiesDialog>['$props']['plugin'],
) {
  const w = mount(PluginCapabilitiesDialog, {
    ...mountOptions(),
    props: { modelValue: false, plugin },
  })
  mounted.push(w)
  return w
}
async function settle(times = 5) {
  for (let i = 0; i < times; i++) await new Promise((r) => setTimeout(r, 10))
}
const buttons = (text: string) =>
  [...document.body.querySelectorAll('button')].filter(
    (b) => b.textContent?.trim() === text,
  ) as HTMLButtonElement[]

function setTextarea(value: string) {
  const ta = document.body.querySelector('textarea') as HTMLTextAreaElement
  ta.value = value
  ta.dispatchEvent(new Event('input'))
}

beforeEach(() => vi.clearAllMocks())
afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('PluginExecuteDialog', () => {
  it('提交 input，展示输出、日志与耗时', async () => {
    api.executePlugin.mockResolvedValue({
      data: {
        output: [],
        output_hex: '',
        output_string: 'Hello, Bob!',
        execution_ms: 3,
        logs: ['[INFO] hi'],
      },
    })
    const w = mountExecute('flame-hello')
    await w.setProps({ modelValue: true })
    await settle()
    setTextarea('Bob')
    await nextTick()
    buttons('运行')[0]!.click()
    await settle()
    expect(api.executePlugin).toHaveBeenCalledWith('flame-hello', 'run', 'Bob')
    const text = document.body.textContent ?? ''
    expect(text).toContain('Hello, Bob!')
    expect(text).toContain('[INFO] hi')
    expect(text).toContain('耗时 3 ms')
    expect(w.emitted('executed')).toHaveLength(1)
  })

  it('执行失败显示错误', async () => {
    api.executePlugin.mockRejectedValue(new Error('WASM execution timed out after 200 ms'))
    const w = mountExecute('p')
    await w.setProps({ modelValue: true })
    await settle()
    buttons('运行')[0]!.click()
    await settle()
    expect(api.executePlugin).toHaveBeenCalledWith('p', 'run', undefined)
    expect(document.body.textContent).toContain('timed out')
  })
})

describe('PluginCapabilitiesDialog', () => {
  const plugin = {
    id: 'p1',
    name: 'P',
    version: '1',
    author: '',
    description: '',
    enabled: true,
    status: 'Loaded',
    loaded_at: '',
    last_executed_at: null,
    exec_count: 0,
    capabilities: ['metrics:read'],
  }

  it('预填当前能力，保存时提交解析后的列表', async () => {
    api.updatePluginCapabilities.mockResolvedValue({ data: plugin })
    const w = mountCapabilities(plugin)
    await w.setProps({ modelValue: true })
    await settle()
    const ta = document.body.querySelector('textarea') as HTMLTextAreaElement
    expect(ta.value).toBe('metrics:read')
    setTextarea('metrics:read\nfs:write:/opt/x')
    await settle()
    expect(document.body.textContent).toContain('写入目录 /opt/x')
    buttons('保存')[0]!.click()
    await settle()
    expect(api.updatePluginCapabilities).toHaveBeenCalledWith('p1', [
      'metrics:read',
      'fs:write:/opt/x',
    ])
    expect(toast.success).toHaveBeenCalled()
    expect(w.emitted('saved')).toHaveLength(1)
  })

  it('后端拒绝（敏感目录）提示错误且不关闭', async () => {
    api.updatePluginCapabilities.mockRejectedValue(new Error('directory is not allowed'))
    const w = mountCapabilities(plugin)
    await w.setProps({ modelValue: true })
    await settle()
    buttons('保存')[0]!.click()
    await settle()
    expect(toast.error).toHaveBeenCalled()
    expect(w.emitted('saved')).toBeUndefined()
  })
})
