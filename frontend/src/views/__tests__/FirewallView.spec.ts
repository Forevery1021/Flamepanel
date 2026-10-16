import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FirewallView from '@/views/FirewallView.vue'
import type { FirewallRule, FirewallStatus } from '@/types'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * S-13 防火墙页：新建 / 编辑共用弹窗的校验与请求体规范化、编辑可清空端口、
 * ICMP 禁用端口、按后端提示限制、未启用的 ufw 不再显示为已启用。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listFirewallRules: vi.fn(),
    getFirewallStatus: vi.fn(),
    createFirewallRule: vi.fn(),
    updateFirewallRule: vi.fn(),
    deleteFirewallRule: vi.fn(),
    toggleFirewallRule: vi.fn(),
    applyFirewallRules: vi.fn(),
    enableFirewall: vi.fn(),
    disableFirewall: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))

vi.mock('@/api/firewall', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

function rule(over: Partial<FirewallRule> = {}): FirewallRule {
  return {
    id: 1,
    name: '允许 SSH',
    description: null,
    protocol: 'tcp',
    port: '22',
    source: '0.0.0.0/0',
    destination: null,
    action: 'allow',
    enabled: true,
    priority: 50,
    direction: 'in',
    created_at: '2026-09-01T00:00:00Z',
    updated_at: '2026-09-01T00:00:00Z',
    ...over,
  }
}

function status(over: Partial<FirewallStatus> = {}): FirewallStatus {
  return { backend: 'Ufw', backend_name: 'ufw', status: 'Status: active', ...over }
}

const LayoutStub = {
  name: 'LayoutContent',
  template: '<div class="stub-layout"><slot name="toolbar" /><slot /></div>',
}

const mounted: { unmount: () => void }[] = []

function mountView() {
  const w = mount(FirewallView, {
    attachTo: document.body,
    global: {
      plugins: [
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
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

const $ = <T extends Element = HTMLElement>(sel: string) => document.body.querySelector<T>(sel)

function input(fieldClass: string): HTMLInputElement {
  const el = $<HTMLInputElement>(`.${fieldClass} input`)
  expect(el, `找不到 ${fieldClass}`).toBeTruthy()
  return el!
}

async function type(fieldClass: string, value: string) {
  const el = input(fieldClass)
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await settle(1)
}

async function submit() {
  $<HTMLButtonElement>('.rule-submit')!.click()
  await settle()
}

beforeEach(() => {
  for (const fn of Object.values(api)) fn.mockReset()
  for (const fn of Object.values(toast)) fn.mockReset()
  api.listFirewallRules.mockResolvedValue({ data: { data: [rule()], total: 1 } })
  api.getFirewallStatus.mockResolvedValue({ data: status() })
  api.createFirewallRule.mockResolvedValue({ data: rule({ id: 2 }) })
  api.updateFirewallRule.mockResolvedValue({ data: rule() })
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
  document.body.innerHTML = ''
})

describe('FirewallView', () => {
  it('新建：非法输入逐项提示且不提交；修正后提交规范化请求体', async () => {
    const w = mountView()
    await settle()
    await w.find('.add-rule').trigger('click')
    await settle(2)

    await type('field-port', '80,70000')
    await type('field-source', '10.0.0.0/33')
    await submit()
    expect(api.createFirewallRule).not.toHaveBeenCalled()
    const text = document.body.textContent ?? ''
    expect(text).toContain('此项为必填')
    expect(text).toContain('无效端口：70000')
    expect(text).toContain('无效前缀长度 /33')

    await type('field-name', '  Web  ')
    await type('field-port', '443, 8000:9000')
    await type('field-source', '192.168.1.10/24')
    expect(document.body.textContent).toContain('将保存为 192.168.1.0/24')
    await submit()

    expect(api.createFirewallRule).toHaveBeenCalledWith({
      name: 'Web',
      description: '',
      protocol: 'tcp',
      port: '443,8000-9000',
      source: '192.168.1.0/24',
      action: 'allow',
      direction: 'in',
      priority: 50,
    })
    expect(toast.success).toHaveBeenCalled()
    expect(api.listFirewallRules).toHaveBeenCalledTimes(2)
  })

  it('编辑：回填表单，清空端口时发送空串（修复无法清空端口）', async () => {
    const w = mountView()
    await settle()
    await w
      .findAll('button')
      .find((b) => b.text() === '编辑')!
      .trigger('click')
    await settle(2)

    expect(input('field-name').value).toBe('允许 SSH')
    expect(input('field-port').value).toBe('22')
    await type('field-port', '')
    await submit()

    expect(api.updateFirewallRule).toHaveBeenCalledWith(
      1,
      expect.objectContaining({ name: '允许 SSH', port: '', source: '0.0.0.0/0' }),
    )
  })

  it('ICMP 规则禁用端口输入；firewalld 出站规则给出后端限制提示', async () => {
    api.listFirewallRules.mockResolvedValue({
      data: {
        data: [rule({ id: 7, name: 'ping', protocol: 'icmp', port: null, direction: 'out' })],
        total: 1,
      },
    })
    api.getFirewallStatus.mockResolvedValue({
      data: status({ backend_name: 'firewalld', status: 'running' }),
    })
    const w = mountView()
    await settle()
    await w
      .findAll('button')
      .find((b) => b.text() === '编辑')!
      .trigger('click')
    await settle(2)

    expect(input('field-port').disabled).toBe(true)
    const text = document.body.textContent ?? ''
    expect(text).toContain('ICMP 与「任意」协议不区分端口')
    expect(text).toContain('目标 IP')
    expect(text).toContain('rich rule 只过滤入站流量')
  })

  it('后端拒绝时展示后端原因（toast.error 携带错误对象）', async () => {
    const err = Object.assign(new Error('port: ports require protocol tcp or udp'), {
      status: 400,
    })
    api.createFirewallRule.mockRejectedValue(err)
    const w = mountView()
    await settle()
    await w.find('.add-rule').trigger('click')
    await settle(2)
    await type('field-name', 'x')
    await submit()
    expect(toast.error).toHaveBeenCalledWith(err, expect.any(String))
    expect($('.rule-submit'), '失败时保留弹窗').toBeTruthy()
  })

  it('未启用的 ufw（Status: inactive）显示为关闭，按钮为「开启防火墙」', async () => {
    api.getFirewallStatus.mockResolvedValue({ data: status({ status: 'Status: inactive' }) })
    const w = mountView()
    await settle()
    expect(w.text()).toContain('开启防火墙')
    expect(w.text()).not.toContain('关闭防火墙')
  })
})
