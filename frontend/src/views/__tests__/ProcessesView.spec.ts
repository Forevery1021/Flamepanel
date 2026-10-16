import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import ProcessesView from '@/views/ProcessesView.vue'
import type { ProcessListResponse, ProcessRow } from '@/api/toolbox'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import zhCN from '@/locales/zh-CN'

/**
 * S-12 进程管理页：列表渲染（面板标记 / 状态 / 脱敏命令行）、受保护行禁用「结束」、
 * 结束进程弹窗（缺省 SIGTERM、带 started_at）、409 专用提示。
 */

const { api, toast } = vi.hoisted(() => ({
  api: { listProcesses: vi.fn(), killProcess: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), show: vi.fn() },
}))

vi.mock('@/api/toolbox', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/toolbox')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

function row(over: Partial<ProcessRow> = {}): ProcessRow {
  return {
    pid: 300,
    ppid: 1,
    name: 'nginx',
    user: 'www-data',
    uid: 33,
    command: '/usr/sbin/nginx -g daemon off;',
    cpu: 12.5,
    memory_bytes: 30 * 1024 * 1024,
    status: 'Run',
    started_at: Math.floor(Date.now() / 1000) - 3 * 3600,
    kernel_thread: false,
    protected: null,
    ...over,
  }
}

function response(): ProcessListResponse {
  return {
    items: [
      row({ pid: 400, name: 'node', cpu: 88, user: 'app', command: 'node server.js --token=***' }),
      row(),
      row({
        pid: 120,
        name: 'flamepanel',
        user: 'flame',
        cpu: 1.2,
        status: 'Sleep',
        command: '/opt/flamepanel/flamepanel',
        protected: 'panel',
      }),
      row({ pid: 1, name: 'systemd', user: 'root', cpu: 0, status: 'Sleep', protected: 'init' }),
    ],
    total: 57,
    matched: 57,
    total_memory_bytes: 512 * 1024 * 1024,
    panel_pid: 120,
    sampled_at: Date.now(),
  }
}

const LayoutStub = {
  name: 'LayoutContent',
  props: {
    title: { type: String, default: '' },
    description: { type: String, default: '' },
    summary: { type: Array, default: () => [] },
  },
  template:
    '<div class="stub-layout"><span v-for="s in summary" :key="s.label" class="kpi">{{ s.label }}:{{ s.value }}</span><slot name="actions" /><slot name="toolbar" /><slot /></div>',
}

const mounted: { unmount: () => void }[] = []

function mountView() {
  const w = mount(ProcessesView, {
    attachTo: document.body,
    global: {
      plugins: [
        createTestI18n(zhCN as unknown as Record<string, unknown>),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { LayoutContent: LayoutStub },
    },
  })
  mounted.push(w)
  return w
}

async function settle(times = 6) {
  for (let i = 0; i < times; i++) {
    await new Promise((r) => setTimeout(r, 10))
  }
}

function killButton(w: ReturnType<typeof mountView>, name: string) {
  const tr = w.findAll('tr').find((r) => r.text().includes(name))
  expect(tr, `找不到 ${name} 所在行`).toBeTruthy()
  return tr!.find('.kill-btn')
}

beforeEach(() => {
  api.listProcesses.mockReset()
  api.killProcess.mockReset()
  for (const fn of Object.values(toast)) fn.mockReset()
})

afterEach(() => {
  while (mounted.length) mounted.pop()!.unmount()
  document.body.innerHTML = ''
})

describe('ProcessesView', () => {
  it('渲染 KPI、进程行、面板标记、状态与脱敏命令行', async () => {
    api.listProcesses.mockResolvedValue({ data: response() })
    const w = mountView()
    await settle()

    expect(api.listProcesses).toHaveBeenCalledWith(
      expect.objectContaining({ sort: 'cpu', order: 'desc', limit: 100 }),
    )
    const kpis = w.findAll('.kpi').map((k) => k.text())
    expect(kpis).toContain('进程总数:57')
    expect(kpis).toContain('常驻内存合计:512 MB')

    const text = w.text()
    expect(text).toContain('node server.js --token=***')
    expect(text).toContain('30 MB')
    expect(text).toContain('3h 0m')
    expect(text).toContain('运行')
    const panelRow = w.findAll('tr').find((r) => r.text().includes('flamepanel'))!
    expect(panelRow.text()).toContain('面板')
    // 高 CPU 着色
    expect(w.find('.tone-danger').text()).toBe('88%')
  })

  it('受保护进程的「结束」按钮禁用并提示原因', async () => {
    api.listProcesses.mockResolvedValue({ data: response() })
    const w = mountView()
    await settle()

    const panelBtn = killButton(w, 'flamepanel')
    expect(panelBtn.attributes('disabled')).toBeDefined()
    expect(panelBtn.attributes('title')).toContain('面板自身进程')
    expect(killButton(w, 'systemd').attributes('title')).toContain('init')
    expect(killButton(w, 'nginx').attributes('disabled')).toBeUndefined()
  })

  it('结束进程：缺省 SIGTERM，携带 started_at，成功后提示', async () => {
    const data = response()
    api.listProcesses.mockResolvedValue({ data })
    api.killProcess.mockResolvedValue({
      data: { pid: 300, name: 'nginx', signal: 'TERM', elevated: true },
    })
    const w = mountView()
    await settle()

    await killButton(w, 'nginx').trigger('click')
    await settle(2)
    const confirm = document.body.querySelector<HTMLButtonElement>('.kill-confirm')
    expect(confirm, '弹窗未打开').toBeTruthy()
    expect(confirm!.textContent).toContain('发送 SIGTERM')
    expect(document.body.textContent).toContain('/usr/sbin/nginx -g daemon off;')
    confirm!.click()
    await settle()

    const nginx = data.items.find((r) => r.pid === 300)!
    expect(api.killProcess).toHaveBeenCalledWith(300, {
      signal: 'TERM',
      started_at: nginx.started_at,
    })
    expect(toast.success).toHaveBeenCalledWith(
      expect.stringContaining('已向 nginx（PID 300）发送 SIGTERM'),
    )
  })

  it('选择 SIGKILL 会显示强制结束警告；409 给出 pid 复用提示并刷新列表', async () => {
    api.listProcesses.mockResolvedValue({ data: response() })
    api.killProcess.mockRejectedValue(Object.assign(new Error('conflict'), { status: 409 }))
    const w = mountView()
    await settle()

    await killButton(w, 'node').trigger('click')
    await settle(2)
    const killRadio = document.body.querySelector<HTMLInputElement>('input[id$="-KILL"]')
    expect(killRadio, '找不到 SIGKILL 选项').toBeTruthy()
    killRadio!.click()
    await settle(2)
    expect(document.body.textContent).toContain('SIGKILL 会立即终止进程')
    document.body.querySelector<HTMLButtonElement>('.kill-confirm')!.click()
    await settle()

    expect(api.killProcess).toHaveBeenCalledWith(400, expect.objectContaining({ signal: 'KILL' }))
    expect(toast.warning).toHaveBeenCalledWith(expect.stringContaining('PID 被复用'))
    expect(api.listProcesses.mock.calls.length).toBeGreaterThanOrEqual(2)
  })

  it('加载失败显示错误态', async () => {
    api.listProcesses.mockRejectedValue(new Error('503'))
    const w = mountView()
    await new Promise((r) => setTimeout(r, 1400))
    await settle(2)
    expect(w.text()).toContain('进程列表加载失败')
  })
})
