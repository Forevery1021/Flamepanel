import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import PluginsView from '@/views/PluginsView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * batch-14 插件页装配：已安装 / 市场两个页签；表格展示宿主能力徽标；
 * 「能力」按钮打开授予对话框；加载插件时把能力文本解析后随请求提交。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listPlugins: vi.fn(),
    loadPlugin: vi.fn(),
    unloadPlugin: vi.fn(),
    enablePlugin: vi.fn(),
    disablePlugin: vi.fn(),
    executePlugin: vi.fn(),
    updatePluginCapabilities: vi.fn(),
    listMarketPlugins: vi.fn(),
    installMarketPlugin: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))
vi.mock('@/api/plugins', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const plugin = {
  id: 'flame-log-viewer',
  name: 'Log Viewer',
  version: '1.0.0',
  author: 'FlamePanel',
  description: '',
  enabled: true,
  status: 'Loaded',
  loaded_at: '',
  last_executed_at: null,
  exec_count: 2,
  capabilities: ['fs:read:/var/log'],
}

const mounted: { unmount: () => void }[] = []
function mountView() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const w = mount(PluginsView, {
    attachTo: document.body,
    global: {
      plugins: [
        pinia,
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          {
            queryClient: new QueryClient({
              defaultOptions: { queries: { retry: false, retryDelay: 0 } },
            }),
          },
        ],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { RouterLink: true },
    },
  })
  mounted.push(w)
  return w
}
async function settle(times = 6) {
  for (let i = 0; i < times; i++) await new Promise((r) => setTimeout(r, 10))
}
const buttons = (text: string) =>
  [...document.body.querySelectorAll('button')].filter(
    (b) => b.textContent?.trim() === text,
  ) as HTMLButtonElement[]

beforeEach(() => {
  vi.clearAllMocks()
  api.listPlugins.mockResolvedValue({ data: [plugin] })
  api.listMarketPlugins.mockResolvedValue({ data: { plugins: [], remote_error: null } })
  api.loadPlugin.mockResolvedValue({ data: plugin })
})
afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('PluginsView', () => {
  it('两个页签；表格展示能力徽标；「能力」打开授予对话框并预填', async () => {
    mountView()
    await settle()
    const text = document.body.textContent ?? ''
    expect(text).toContain('已安装')
    expect(text).toContain('插件市场')
    expect(text).toContain('fs:read:/var/log')
    buttons('能力')[0]!.click()
    await settle()
    const ta = [...document.body.querySelectorAll('textarea')].find(
      (t) => t.value === 'fs:read:/var/log',
    )
    expect(ta).toBeTruthy()
  })

  it('加载插件时提交解析后的能力列表', async () => {
    mountView()
    await settle()
    buttons('加载插件')[0]!.click()
    await settle()
    const inputs = [
      ...document.body.querySelectorAll('.fp-modal input, [role="dialog"] input'),
    ] as HTMLInputElement[]
    const areas = [
      ...document.body.querySelectorAll('[role="dialog"] textarea'),
    ] as HTMLTextAreaElement[]
    const set = (el: HTMLInputElement | HTMLTextAreaElement, v: string) => {
      el.value = v
      el.dispatchEvent(new Event('input'))
    }
    set(inputs[0]!, 'p2')
    set(inputs[1]!, 'P2')
    set(areas[0]!, 'AGFzbQEAAAA=')
    set(areas[areas.length - 1]!, 'metrics:read\nmetrics:read')
    await settle()
    const loadButtons = buttons('加载插件')
    loadButtons[loadButtons.length - 1]!.click()
    await settle()
    expect(api.loadPlugin).toHaveBeenCalledWith('p2', 'P2', 'AGFzbQEAAAA=', {
      version: '0.1.0',
      author: '',
      description: '',
      capabilities: ['metrics:read'],
    })
  })
})
