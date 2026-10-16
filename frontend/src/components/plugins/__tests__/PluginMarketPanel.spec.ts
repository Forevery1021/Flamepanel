import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import PluginMarketPanel from '@/components/plugins/PluginMarketPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/** batch-14 插件市场：能力徽标 / 申请能力须经授权确认 / 无能力直接安装 / 远程失败提示 */

const { api, toast } = vi.hoisted(() => ({
  api: { listMarketPlugins: vi.fn(), installMarketPlugin: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))
vi.mock('@/api/plugins', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const entry = (over: Record<string, unknown> = {}) => ({
  id: 'flame-hello',
  name: 'Hello',
  version: '1.0.0',
  author: 'FlamePanel',
  description: 'Greets',
  capabilities: [],
  tags: [],
  entry: 'run',
  wasm_sha256: 'a'.repeat(64),
  source: 'builtin',
  installed: false,
  ...over,
})

const mounted: { unmount: () => void }[] = []
function mountPanel() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const w = mount(PluginMarketPanel, {
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
  api.listMarketPlugins.mockResolvedValue({
    data: {
      plugins: [
        entry(),
        entry({ id: 'flame-log-viewer', name: 'Log Viewer', capabilities: ['fs:read:/var/log'] }),
        entry({
          id: 'flame-metrics-snapshot',
          name: 'Metrics',
          installed: true,
          installed_version: '1.0.0',
        }),
      ],
      remote_url: null,
      remote_error: null,
    },
  })
  api.installMarketPlugin.mockResolvedValue({ data: {} })
})
afterEach(() => {
  mounted.splice(0).forEach((w) => w.unmount())
  document.body.innerHTML = ''
})

describe('PluginMarketPanel', () => {
  it('展示目录、能力徽标与安装状态', async () => {
    mountPanel()
    await settle()
    const text = document.body.textContent ?? ''
    expect(text).toContain('Log Viewer')
    expect(text).toContain('fs:read:/var/log')
    expect(text).toContain('无（仅基础 API）')
    expect(text).toContain('内置')
    expect(buttons('安装')).toHaveLength(2)
    expect(text).toContain('已安装')
  })

  it('无能力插件直接安装', async () => {
    const w = mountPanel()
    await settle()
    buttons('安装')[0]!.click()
    await settle()
    expect(api.installMarketPlugin).toHaveBeenCalledWith('flame-hello', false)
    expect(toast.success).toHaveBeenCalled()
    expect(w.emitted('installed')?.[0]).toEqual(['flame-hello'])
  })

  it('申请能力的插件先弹授权确认，同意后才带 accept=true 安装', async () => {
    mountPanel()
    await settle()
    buttons('安装')[1]!.click()
    await settle()
    expect(api.installMarketPlugin).not.toHaveBeenCalled()
    const text = document.body.textContent ?? ''
    expect(text).toContain('授权确认')
    expect(text).toContain('读取目录 /var/log')
    buttons('同意并安装')[0]!.click()
    await settle()
    expect(api.installMarketPlugin).toHaveBeenCalledWith('flame-log-viewer', true)
  })

  it('安装失败提示后端错误；远程索引失败显示告警', async () => {
    api.listMarketPlugins.mockResolvedValue({
      data: { plugins: [entry()], remote_url: 'https://x', remote_error: 'HTTP 404' },
    })
    api.installMarketPlugin.mockRejectedValue({ response: { data: { error: 'Plugin exists' } } })
    mountPanel()
    await settle()
    expect(document.body.textContent).toContain('远程市场不可用：HTTP 404')
    buttons('安装')[0]!.click()
    await settle()
    expect(toast.error).toHaveBeenCalled()
  })
})
