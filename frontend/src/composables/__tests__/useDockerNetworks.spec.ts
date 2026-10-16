import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站的数据层单测（`DockerView` → `useDockerNetworks`）。
 *
 * 本站唯一带**状态相关分支**的数据层：断开连接作用于「第一个已连接容器」，
 * 且无已连接容器时**本地提示、不发请求**。这条分支很容易在重构时被简化成
 * 「直接调接口」，从而把一条必然失败的错误甩给用户——故重点锁死。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listNetworks: vi.fn(),
    createNetwork: vi.fn(),
    removeNetwork: vi.fn(),
    connectNetwork: vi.fn(),
    disconnectNetwork: vi.fn(),
    pruneNetworks: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: {
    last: null as { accept?: () => void; reject?: () => void; message: string } | null,
  },
}))

vi.mock('@/api/docker', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/docker')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; reject?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useDockerNetworks } from '@/composables/useDockerNetworks'
import type { DockerNetwork } from '@/types'

const messages = {
  docker: {
    removeConfirm: '确定要删除 {name} 吗？',
    pruneConfirm: '确定要清理未使用的{what}吗？',
    networks: '网络',
    noContainers: '暂无容器',
  },
  common: { success: '操作成功', failed: '操作失败', confirmAction: '确定执行此操作吗？' },
}

const network = (over: Partial<DockerNetwork> = {}): DockerNetwork => ({
  id: 'net1',
  name: 'bridge',
  driver: 'bridge',
  scope: 'local',
  internal: false,
  attachable: true,
  created: null,
  containers: [],
  ...over,
})

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useDockerNetworks() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: {
      plugins: [
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {} },
    },
  })
  return (w.vm as unknown as { ctl: ReturnType<typeof useDockerNetworks> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listNetworks.mockResolvedValue({ data: [network()] })
  api.createNetwork.mockResolvedValue({ data: null })
  api.removeNetwork.mockResolvedValue({ data: null })
  api.connectNetwork.mockResolvedValue({ data: null })
  api.disconnectNetwork.mockResolvedValue({ data: null })
  api.pruneNetworks.mockResolvedValue({ data: null })
})

describe('useDockerNetworks · 创建', () => {
  it('成功：名称 trim、子网空则不上报（空串会被后端当非法 CIDR），返回 true', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.create({ name: '  app-net  ', driver: 'bridge', subnet: '   ' })
    expect(ok).toBe(true)
    expect(api.createNetwork).toHaveBeenCalledWith('app-net', 'bridge', undefined)
  })

  it('子网填写时原样上报', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    await ctl.create({ name: 'app-net', driver: 'overlay', subnet: '10.0.0.0/24' })
    expect(api.createNetwork).toHaveBeenCalledWith('app-net', 'overlay', '10.0.0.0/24')
  })

  it('失败：返回 false 且创建态复位（弹窗保持打开让用户改输入）', async () => {
    api.createNetwork.mockRejectedValue(new Error('already exists'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.create({ name: 'dup', driver: 'bridge', subnet: '' })
    expect(ok).toBe(false)
    expect(ctl.creating.value).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useDockerNetworks · 连接 / 断开', () => {
  it('连接：空容器 ID 直接返回 false 且不发请求（本地双保险）', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.connect('net1', '')
    expect(ok).toBe(false)
    expect(api.connectNetwork).not.toHaveBeenCalled()
  })

  it('连接成功：按 (网络 ID, 容器 ID) 上报并刷新', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listNetworks.mock.calls.length

    const ok = await ctl.connect('net1', 'c1')
    expect(ok).toBe(true)
    expect(api.connectNetwork).toHaveBeenCalledWith('net1', 'c1')
    expect(api.listNetworks.mock.calls.length).toBe(before + 1)
  })

  it('断开：无已连接容器时**本地提示且不发请求**（避免把必然失败甩给用户）', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    await ctl.disconnect(network({ containers: [] }))
    expect(api.disconnectNetwork).not.toHaveBeenCalled()
    expect(toast.info).toHaveBeenCalledWith('暂无容器')
  })

  it('断开：作用于第一个已连接容器，按容器名寻址', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    await ctl.disconnect(
      network({
        containers: [
          { name: 'web', ipv4_address: '172.20.0.2', ipv6_address: null },
          { name: 'db', ipv4_address: null, ipv6_address: null },
        ],
      }),
    )
    expect(api.disconnectNetwork).toHaveBeenCalledWith('net1', 'web')
  })

  it('断开失败：toast 错误', async () => {
    api.disconnectNetwork.mockRejectedValue(new Error('not connected'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    await ctl.disconnect(
      network({ containers: [{ name: 'web', ipv4_address: null, ipv6_address: null }] }),
    )
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useDockerNetworks · 删除与清理', () => {
  it('删除：确认文案带网络名，确认后删除', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.remove(network({ name: 'app-net' }))
    expect(confirmState.last?.message).toBe('确定要删除 app-net 吗？')
    expect(api.removeNetwork).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.removeNetwork).toHaveBeenCalledWith('net1')
  })

  it('清理：文案为「网络」目标名，确认后 prune', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.prune()
    expect(confirmState.last?.message).toBe('确定要清理未使用的网络吗？')

    confirmState.last?.accept?.()
    await settle()
    expect(api.pruneNetworks).toHaveBeenCalledTimes(1)
  })
})
