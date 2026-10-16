import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const api = vi.hoisted(() => ({
  detectNativeWebServers: vi.fn(),
  nativeInstallWebServer: vi.fn(),
  nativeUninstallWebServer: vi.fn(),
  nativeAutostartWebServer: vi.fn(),
  getEngineCapabilities: vi.fn(),
  installCapabilities: vi.fn(),
}))
vi.mock('@/api/webServers', () => api)

const confirmAction = vi.fn()
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction }) }))
const toast = { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import WebServerNativeTab from '@/components/webserver/WebServerNativeTab.vue'

/**
 * 「本机原生」页签（从 WebServersView 拆出）：
 * 挂载即探测；安装经确认后调用接口、刷新列表并 emit changed（父级刷新面板实例）；
 * 失败把后端错误交给 toast（不再吞成「失败」）；配套能力只提交勾选项，部分成功给出警告。
 */
const rows = [
  { engine: 'nginx', installed: false, running: false, enabled: false, listening_ports: [] },
  { engine: 'caddy', installed: true, running: true, enabled: true, listening_ports: [80, 443] },
]

function mountTab() {
  document.body.innerHTML = ''
  return mount(WebServerNativeTab, {
    attachTo: document.body,
    global: {
      plugins: [createTestI18n({}), openVuePlugin, ...services],
      directives: { tooltip: () => {} },
    },
  })
}

function button(label: string): HTMLButtonElement {
  const b = Array.from(document.body.querySelectorAll('button')).find(
    (x) => x.textContent?.trim() === label,
  )
  if (!b) throw new Error(`no button ${label}`)
  return b as HTMLButtonElement
}

beforeEach(() => {
  vi.clearAllMocks()
  api.detectNativeWebServers.mockResolvedValue({ data: rows })
})

describe('WebServerNativeTab', () => {
  it('挂载即探测并渲染引擎行', async () => {
    const w = mountTab()
    await flushPromises()
    expect(api.detectNativeWebServers).toHaveBeenCalledTimes(1)
    expect(w.text()).toContain('nginx')
    expect(w.text()).toContain('443')
  })

  it('安装：确认后调用接口、重新探测并 emit changed', async () => {
    api.nativeInstallWebServer.mockResolvedValue({ data: null })
    const w = mountTab()
    await flushPromises()
    button('webServer.nativeInstallBtn').click()
    expect(confirmAction).toHaveBeenCalledTimes(1)
    await confirmAction.mock.calls[0][0].accept()
    await flushPromises()
    expect(api.nativeInstallWebServer).toHaveBeenCalledWith('nginx')
    expect(api.detectNativeWebServers).toHaveBeenCalledTimes(2)
    expect(w.emitted('changed')).toHaveLength(1)
  })

  it('卸载失败：后端错误交给 toast，不 emit changed', async () => {
    const err = new Error('apt locked')
    api.nativeUninstallWebServer.mockRejectedValue(err)
    const w = mountTab()
    await flushPromises()
    button('webServer.nativeUninstallBtn').click()
    await confirmAction.mock.calls[0][0].accept()
    await flushPromises()
    expect(toast.error).toHaveBeenCalledWith(err, expect.any(String))
    expect(w.emitted('changed')).toBeUndefined()
  })

  it('配套能力：只提交勾选项，部分成功给出警告', async () => {
    api.getEngineCapabilities.mockResolvedValue({
      data: {
        capabilities: [
          { key: 'brotli', label_zh: 'Brotli', packages: ['libnginx-mod-brotli'], hint: '' },
          { key: 'geoip', label_zh: 'GeoIP', packages: [], hint: '' },
        ],
      },
    })
    api.installCapabilities.mockResolvedValue({
      data: { success: false, installed: ['brotli'], failed: [] },
    })
    mountTab()
    await flushPromises()
    button('webServer.capabilities').click()
    await flushPromises()
    const boxes = document.body.querySelectorAll<HTMLInputElement>(
      '.cap-list input[type="checkbox"]',
    )
    expect(boxes).toHaveLength(2)
    boxes[0].click()
    await flushPromises()
    button('common.install').click()
    await flushPromises()
    expect(api.installCapabilities).toHaveBeenCalledWith('nginx', ['brotli'])
    expect(toast.warning).toHaveBeenCalled()
  })
})
