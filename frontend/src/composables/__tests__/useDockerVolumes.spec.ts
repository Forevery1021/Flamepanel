import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站的数据层单测（`DockerView` → `useDockerVolumes`）。
 *
 * 卷是三张「简单表」里唯一**按名字寻址**的（`removeVolume(name)`）——Docker API
 * 的语义如此。这条差异拆出后在签名上直接可见，本 spec 把它固化下来：
 * 将来谁把 `row.name` 改成 `row.id`，用例会立刻红。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listVolumes: vi.fn(),
    createVolume: vi.fn(),
    removeVolume: vi.fn(),
    pruneVolumes: vi.fn(),
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

import { useDockerVolumes } from '@/composables/useDockerVolumes'

const messages = {
  docker: {
    removeConfirm: '确定要删除 {name} 吗？',
    pruneConfirm: '确定要清理未使用的{what}吗？',
    volumes: '卷',
  },
  common: { success: '操作成功', failed: '操作失败', confirmAction: '确定执行此操作吗？' },
}

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useDockerVolumes() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useDockerVolumes> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listVolumes.mockResolvedValue({
    data: [{ name: 'app-data', driver: 'local', mountpoint: '/var/lib/docker/volumes/app-data' }],
  })
  api.createVolume.mockResolvedValue({ data: null })
  api.removeVolume.mockResolvedValue({ data: null })
  api.pruneVolumes.mockResolvedValue({ data: null })
})

describe('useDockerVolumes', () => {
  it('惰性启用：初始不发请求，置 true 后加载', async () => {
    const ctl = mountCtl()
    await settle()
    expect(api.listVolumes).not.toHaveBeenCalled()

    ctl.enabled.value = true
    await settle()
    expect(api.listVolumes).toHaveBeenCalledTimes(1)
    expect(ctl.volumes.value).toHaveLength(1)
  })

  it('创建：名称 trim 后上报，成功返回 true', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.create({ name: '  data-2  ', driver: 'local' })
    expect(ok).toBe(true)
    expect(api.createVolume).toHaveBeenCalledWith('data-2', 'local')
  })

  it('创建失败：返回 false 且创建态复位', async () => {
    api.createVolume.mockRejectedValue(new Error('exists'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.create({ name: 'dup', driver: 'local' })
    expect(ok).toBe(false)
    expect(ctl.creating.value).toBe(false)
  })

  it('删除：**按名字**寻址（Docker API 语义，不是 ID）', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.remove({ name: 'app-data', driver: 'local', mountpoint: '/v', created_at: null })
    expect(confirmState.last?.message).toBe('确定要删除 app-data 吗？')

    confirmState.last?.accept?.()
    await settle()
    expect(api.removeVolume).toHaveBeenCalledWith('app-data')
  })

  it('清理：文案为「卷」目标名，确认后 prune 并刷新', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listVolumes.mock.calls.length

    ctl.prune()
    expect(confirmState.last?.message).toBe('确定要清理未使用的卷吗？')

    confirmState.last?.accept?.()
    await settle()
    expect(api.pruneVolumes).toHaveBeenCalledTimes(1)
    expect(api.listVolumes.mock.calls.length).toBe(before + 1)
  })
})
