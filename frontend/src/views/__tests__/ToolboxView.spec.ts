import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import ToolboxView from '@/views/ToolboxView.vue'
import type { HostDetails } from '@/api/toolbox'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * 系统工具箱页：工具入口按角色过滤 + 主机详情（系统 / 硬件 / 时间 / 网络 / DNS / hosts / 磁盘）。
 */

const { api } = vi.hoisted(() => ({ api: { getHostDetails: vi.fn() } }))

vi.mock('@/api/toolbox', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/toolbox')>()
  return { ...actual, ...api }
})

function host(over: Partial<HostDetails> = {}): HostDetails {
  const now = new Date().toISOString()
  return {
    generated_at: now,
    system: {
      hostname: 'web-01',
      os_name: 'Ubuntu 24.04 LTS',
      os_id: 'ubuntu',
      os_version_id: '24.04',
      kernel_release: '6.8.0-45-generic',
      arch: 'x86_64',
      uptime_secs: 2 * 86_400 + 5 * 3_600,
      boot_time: '2026-09-29T00:00:00Z',
    },
    cpu: {
      model: 'Intel(R) Xeon(R) Gold 6148',
      logical_cores: 8,
      physical_cores: 4,
      load_average: [0.5, 0.25, 0.125],
    },
    memory: {
      total_bytes: 8 * 1024 ** 3,
      available_bytes: 2 * 1024 ** 3,
      swap_total_bytes: 0,
      swap_used_bytes: 0,
    },
    time: { timezone: 'Asia/Shanghai', utc_offset: '+08:00', now },
    dns: { nameservers: ['223.5.5.5', '1.1.1.1'], search: ['lan'], options: [] },
    hosts: [
      { ip: '127.0.0.1', names: ['localhost'] },
      { ip: '10.0.0.9', names: ['db', 'db.lan'] },
    ],
    hosts_truncated: false,
    interfaces: [
      {
        name: 'eth0',
        mac: '52:54:00:aa:bb:cc',
        addresses: ['10.0.0.2/24', 'fe80::1/64'],
        rx_bytes: 1024,
        tx_bytes: 2048,
        virtual_iface: false,
      },
      {
        name: 'docker0',
        mac: null,
        addresses: ['172.17.0.1/16'],
        rx_bytes: 0,
        tx_bytes: 0,
        virtual_iface: true,
      },
    ],
    disks: [
      {
        device: '/dev/vda1',
        mount_point: '/',
        fs_type: 'ext4',
        total_bytes: 100 * 1024 ** 3,
        available_bytes: 5 * 1024 ** 3,
      },
    ],
    panel: { version: '1.0.0', pid: 4242, uid: 0, running_as_root: true },
    ...over,
  }
}

const LayoutStub = {
  name: 'LayoutContent',
  props: { title: { type: String, default: '' }, summary: { type: Array, default: () => [] } },
  template:
    '<div class="stub-layout" :data-title="title"><span v-for="s in summary" :key="s.label" class="stub-summary">{{ s.label }}={{ s.value }}</span><slot /></div>',
}

const RouterLinkStub = {
  name: 'RouterLink',
  props: { to: { type: String, required: true } },
  template: '<a :href="to"><slot /></a>',
}

const mounted: { unmount: () => void }[] = []

function mountView(role = 'admin') {
  const pinia = createPinia()
  setActivePinia(pinia)
  useAuthStore().role = role
  const w = mount(ToolboxView, {
    global: {
      plugins: [
        pinia,
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { LayoutContent: LayoutStub, RouterLink: RouterLinkStub },
    },
  })
  mounted.push(w)
  return w
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) await new Promise((r) => setTimeout(r, 10))
}

beforeEach(() => {
  api.getHostDetails.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
})

describe('ToolboxView', () => {
  it('渲染工具入口与主机详情各分区', async () => {
    api.getHostDetails.mockResolvedValue({ data: host() })
    const w = mountView()
    await settle()

    const tools = w.findAll('.tool-card').map((a) => a.attributes('href'))
    expect(tools).toEqual([
      '/processes',
      '/inspection',
      '/terminal',
      '/firewall',
      '/scheduled-tasks',
      '/system-logs',
      '/health',
    ])

    const text = w.text()
    // 摘要
    expect(text).toContain('主机名=web-01')
    expect(text).toContain('运行时长=2d 5h')
    // 系统 / 硬件
    expect(text).toContain('Ubuntu 24.04 LTS')
    expect(text).toContain('6.8.0-45-generic')
    expect(text).toContain('Intel(R) Xeon(R) Gold 6148')
    expect(text).toContain('8 逻辑 / 4 物理')
    expect(text).toContain('0.50 / 0.25 / 0.13')
    expect(text).toContain('75%')
    expect(text).toContain('未启用 Swap')
    // 时间
    expect(text).toContain('Asia/Shanghai')
    expect(text).toContain('UTC+08:00')
    expect(text).toContain('时钟正常')
    // 面板以 root 运行
    expect(text).toContain('以 root 运行')
    // DNS / hosts
    expect(text).toContain('223.5.5.5, 1.1.1.1')
    expect(text).toContain('db db.lan')
    // 磁盘（95% 已用 → danger）
    expect(w.find('.disk-item').text()).toContain('/dev/vda1 · ext4')
    expect(w.find('.disk-item .usage-bar').classes()).toContain('tone-danger')
  })

  it('默认隐藏虚拟网卡，打开开关后显示', async () => {
    api.getHostDetails.mockResolvedValue({ data: host() })
    const w = mountView()
    await settle()

    const names = () => w.findAll('.kv-table tbody tr td:first-child').map((td) => td.text())
    expect(names()).toEqual(['eth0'])
    expect(w.text()).toContain('显示虚拟网卡（1）')

    await w.find('.fp-switch input').setValue(true)
    await settle(2)
    expect(names()[1]).toContain('docker0')
  })

  it('viewer 看不到需要执行权限的终端入口', async () => {
    api.getHostDetails.mockResolvedValue({ data: host() })
    const w = mountView('viewer')
    await settle()
    const tools = w.findAll('.tool-card').map((a) => a.attributes('href'))
    expect(tools).not.toContain('/terminal')
    expect(tools).toContain('/processes')
  })

  it('时钟偏差超过阈值时提示', async () => {
    const skewed = new Date(Date.now() + 5 * 60_000).toISOString()
    api.getHostDetails.mockResolvedValue({
      data: host({ time: { timezone: null, utc_offset: '+00:00', now: skewed } }),
    })
    const w = mountView()
    await settle()
    expect(w.text()).toContain('时钟偏差')
    expect(w.text()).toContain('建议启用 NTP 时间同步')
    expect(w.text()).toContain('不可探测')
  })

  it('加载失败显示错误态', async () => {
    api.getHostDetails.mockRejectedValue(new Error('boom'))
    const w = mountView()
    // useHostDetails 自带 retry: 1（默认退避约 1 s），等重试也失败后再断言
    await new Promise((r) => setTimeout(r, 1_300))
    await settle()
    expect(w.text()).toContain('主机详情加载失败')
  })
})
