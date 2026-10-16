import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站的数据层单测（`DockerView` → `useDockerImages`）。
 *
 * 三条要点：① **惰性启用**——初始 enabled=false 时不发请求，置 true 才拉；
 * ② 拉取的提示文案优先透出**后端回显**（长操作里那句「成功」太笼统）；
 * ③ 删除/清理的二次确认分支。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: { listImages: vi.fn(), removeImage: vi.fn(), pullImage: vi.fn(), pruneImages: vi.fn() },
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

import { useDockerImages } from '@/composables/useDockerImages'

const messages = {
  docker: {
    removeConfirm: '确定要删除 {name} 吗？',
    pruneConfirm: '确定要清理未使用的{what}吗？',
    images: '镜像',
  },
  common: { success: '操作成功', failed: '操作失败', confirmAction: '确定执行此操作吗？' },
}

function makeHost() {
  return defineComponent({
    setup() {
      return { ctl: useDockerImages() }
    },
    template: '<div/>',
  })
}

function mountCtl() {
  const w = mount(makeHost(), {
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useDockerImages> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listImages.mockResolvedValue({ data: [{ id: 'img1', tags: ['nginx:latest'], size: 1024 }] })
  api.removeImage.mockResolvedValue({ data: null })
  api.pullImage.mockResolvedValue({ data: 'Pulling from library/nginx' })
  api.pruneImages.mockResolvedValue({ data: null })
})

describe('useDockerImages · 惰性启用', () => {
  it('初始 enabled=false：挂载后不发请求（其余页签首次切换才加载）', async () => {
    mountCtl()
    await settle()
    expect(api.listImages).not.toHaveBeenCalled()
  })

  it('置 enabled=true 触发首次加载（视图层切换页签时的动作）', async () => {
    const ctl = mountCtl()
    await settle()

    ctl.enabled.value = true
    await settle()
    expect(api.listImages).toHaveBeenCalledTimes(1)
    expect(ctl.images.value).toHaveLength(1)
  })
})

describe('useDockerImages · 拉取', () => {
  it('空输入：不发请求、返回 false', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.pull()
    expect(ok).toBe(false)
    expect(api.pullImage).not.toHaveBeenCalled()
  })

  it('成功：镜像名 trim 后上报；提示优先透出后端回显（长操作需要看到实际流水）', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listImages.mock.calls.length

    ctl.pullName.value = '  nginx:alpine  '
    const ok = await ctl.pull()

    expect(ok).toBe(true)
    expect(api.pullImage).toHaveBeenCalledWith('nginx:alpine')
    expect(toast.success).toHaveBeenCalledWith('Pulling from library/nginx')
    expect(api.listImages.mock.calls.length).toBe(before + 1)
  })

  it('后端未回显字符串时回落通用成功文案', async () => {
    api.pullImage.mockResolvedValue({ data: { ok: true } })
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.pullName.value = 'nginx'
    await ctl.pull()
    expect(toast.success).toHaveBeenCalledWith('操作成功')
  })

  it('失败：返回 false 且拉取态复位（按钮不再转圈）', async () => {
    api.pullImage.mockRejectedValue(new Error('pull access denied'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.pullName.value = 'private/x'
    const ok = await ctl.pull()
    expect(ok).toBe(false)
    expect(ctl.pulling.value).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useDockerImages · 删除与清理', () => {
  it('删除：确认文案带镜像 ID，确认前不发请求，确认后删除并刷新', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listImages.mock.calls.length

    ctl.remove({ id: 'img1' })
    expect(confirmState.last?.message).toBe('确定要删除 img1 吗？')
    expect(api.removeImage).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.removeImage).toHaveBeenCalledWith('img1')
    expect(api.listImages.mock.calls.length).toBe(before + 1)
  })

  it('清理：文案为「镜像」目标名，确认后 prune', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    ctl.prune()
    expect(confirmState.last?.message).toBe('确定要清理未使用的镜像吗？')

    confirmState.last?.accept?.()
    await settle()
    expect(api.pruneImages).toHaveBeenCalledTimes(1)
  })

  it('删除失败：toast 错误且不刷新列表', async () => {
    api.removeImage.mockRejectedValue(new Error('image is in use'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listImages.mock.calls.length

    ctl.remove({ id: 'img1' })
    confirmState.last?.accept?.()
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(api.listImages.mock.calls.length).toBe(before)
  })
})
