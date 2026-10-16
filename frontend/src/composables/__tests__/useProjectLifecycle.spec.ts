import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站的生命周期动作单测（`ProjectView` → `useProjectLifecycle`）。
 *
 * 原 832 行里，「按钮 + 请求 + toast + 刷新」这四件事重复了七遍，其中最容易被
 * 「顺手统一」抹平的差异正是本 spec 的重点：
 *
 * ① **成败判定口径不同**：五类动作按「请求成功即成功」，健康探测却要看
 *    响应体的 `healthy`（接口 200 而站点不健康是这条链路的正常返回形态）；
 * ② **失败不刷新**（`withActing` 的容错壳）：失败也刷新会用一次额外请求掩盖首因；
 * ③ **`actingId` 是「行 × 动作」二维锁**：只锁发起动作的那一个按钮；
 * ④ **Webhook 两条路径**：已开启走二次确认轮换，未开启直接开启，而令牌明文
 *    只有这一次响应有——成功处理必须同源。
 */

const { api, toast, confirmState, clipboard } = vi.hoisted(() => ({
  api: {
    deployProject: vi.fn(),
    startProject: vi.fn(),
    stopProject: vi.fn(),
    restartProject: vi.fn(),
    deleteProject: vi.fn(),
    checkProjectHealth: vi.fn(),
    enableProjectWebhook: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; message: string } | null },
  clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
}))

vi.mock('@/api/projects', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/projects')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useProjectLifecycle } from '@/composables/useProjectLifecycle'

const messages = {
  project: {
    deleteConfirm: '确定要删除项目 {name} 吗？运行中项目将自动停止（部署目录保留）',
    deleteSuccess: '项目删除成功',
    healthOk: '健康检查通过',
    healthFail: '健康检查未通过',
    webhookReset: '重置令牌',
    webhookResetConfirm: '确定要重置项目 {name} 的部署令牌吗？旧令牌将立即失效。',
  },
  common: { success: '操作成功', failed: '操作失败', delete: '删除', copySuccess: '已复制' },
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'blog',
  deploy_type: 'manual',
  status: 'stopped',
  source: 'git',
  ...over,
})

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useProjectLifecycle() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useProjectLifecycle> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  Object.defineProperty(navigator, 'clipboard', { value: clipboard, configurable: true })
  api.deployProject.mockResolvedValue({ data: row() })
  api.startProject.mockResolvedValue({ data: row({ status: 'running' }) })
  api.stopProject.mockResolvedValue({ data: row() })
  api.restartProject.mockResolvedValue({ data: row() })
  api.deleteProject.mockResolvedValue({ data: { ok: true } })
  api.checkProjectHealth.mockResolvedValue({ data: { healthy: true, project_id: 3 } })
  api.enableProjectWebhook.mockResolvedValue({
    data: {
      token: 'tok-123',
      webhook_url: '/api/projects/3/webhook?token=tok-123',
      project: row(),
    },
  })
})

describe('useProjectLifecycle · 统一容错壳', () => {
  it('部署成功：toast 成功 + 刷新', async () => {
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.deploy(row() as never, refresh)
    await settle()

    expect(api.deployProject).toHaveBeenCalledWith(3)
    expect(toast.success).toHaveBeenCalledWith('操作成功')
    expect(refresh).toHaveBeenCalled()
  })

  it('【关键】失败：toast 错误且**不刷新**（不让额外请求掩盖首因）', async () => {
    api.deployProject.mockRejectedValue(new Error('git clone 超时'))
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.deploy(row() as never, refresh)
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(refresh).not.toHaveBeenCalled()
  })

  it('四条动作各自打自己的请求（启 / 停 / 重启）', async () => {
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.start(row() as never, refresh)
    await settle()
    ctl.stop(row() as never, refresh)
    await settle()
    ctl.restart(row() as never, refresh)
    await settle()

    expect(api.startProject).toHaveBeenCalledWith(3)
    expect(api.stopProject).toHaveBeenCalledWith(3)
    expect(api.restartProject).toHaveBeenCalledWith(3)
    expect(refresh).toHaveBeenCalledTimes(3)
  })

  it('【关键】actingId 是「行 × 动作」二维锁：只锁发起动作的那一个按钮', async () => {
    let release: () => void = () => {}
    api.deployProject.mockImplementation(
      () => new Promise((resolve) => (release = () => resolve({ data: row() }))),
    )
    const ctl = mountCtl()

    ctl.deploy(row({ id: 9 }) as never, vi.fn())
    await settle()
    // 只有 deploy-9 被锁：同一行的 restart-9、别的行的 deploy-8 都不受影响
    expect(ctl.actingId.value).toBe('deploy-9')

    release()
    await settle()
    expect(ctl.actingId.value).toBe('')
  })
})

describe('useProjectLifecycle · 健康探测与删除', () => {
  it('【关键】healthy=true → 报通过；不刷新列表（探测不改项目状态）', async () => {
    const ctl = mountCtl()
    ctl.healthCheck(row() as never)
    await settle()

    expect(api.checkProjectHealth).toHaveBeenCalledWith(3)
    expect(toast.success).toHaveBeenCalledWith('健康检查通过')
  })

  it('【关键】接口 200 但 healthy=false → 报未通过（不能按「请求成功即成功」）', async () => {
    api.checkProjectHealth.mockResolvedValue({ data: { healthy: false, project_id: 3 } })
    const ctl = mountCtl()
    ctl.healthCheck(row() as never)
    await settle()

    expect(toast.error).toHaveBeenCalledWith('健康检查未通过')
    expect(toast.success).not.toHaveBeenCalled()
  })

  it('探测失败：toast 错误且探测态复位', async () => {
    api.checkProjectHealth.mockRejectedValue(new Error('项目未运行'))
    const ctl = mountCtl()
    ctl.healthCheck(row() as never)
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(ctl.actingId.value).toBe('')
  })

  it('删除：确认文案带项目名，确认后才请求', async () => {
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.remove(row({ name: 'blog' }) as never, refresh)
    expect(confirmState.last?.message).toBe(
      '确定要删除项目 blog 吗？运行中项目将自动停止（部署目录保留）',
    )
    expect(api.deleteProject).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.deleteProject).toHaveBeenCalledWith(3)
    expect(refresh).toHaveBeenCalled()
  })
})

describe('useProjectLifecycle · Webhook 令牌', () => {
  it('未开启：直接开启，令牌写入并弹窗', async () => {
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.toggleWebhook(row({ deploy_type: 'manual' }) as never, refresh)
    await settle()

    expect(api.enableProjectWebhook).toHaveBeenCalledWith(3)
    expect(ctl.webhookToken.value).toBe('tok-123')
    expect(ctl.webhookUrl.value).toBe('/api/projects/3/webhook?token=tok-123')
    expect(ctl.webhookVisible.value).toBe(true)
    expect(refresh).toHaveBeenCalled()
  })

  it('【关键】已开启：先二次确认（轮换令牌，旧令牌即时失效），确认后才请求', async () => {
    const ctl = mountCtl()
    const refresh = vi.fn()

    ctl.toggleWebhook(row({ deploy_type: 'webhook' }) as never, refresh)
    expect(confirmState.last?.message).toBe('确定要重置项目 blog 的部署令牌吗？旧令牌将立即失效。')
    expect(api.enableProjectWebhook).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.enableProjectWebhook).toHaveBeenCalledWith(3)
    expect(ctl.webhookVisible.value).toBe(true)
  })

  it('开启失败：toast 错误且**不弹窗**（不显示一个空令牌窗）', async () => {
    api.enableProjectWebhook.mockRejectedValue(new Error('项目不存在'))
    const ctl = mountCtl()

    ctl.toggleWebhook(row() as never, vi.fn())
    await settle()

    expect(toast.error).toHaveBeenCalled()
    expect(ctl.webhookVisible.value).toBe(false)
  })

  it('【关键】复制的是绝对地址（origin + 相对路径）', async () => {
    const ctl = mountCtl()
    ctl.toggleWebhook(row() as never, vi.fn())
    await settle()

    await ctl.copyWebhookUrl()
    expect(clipboard.writeText).toHaveBeenCalledWith(
      `${window.location.origin}/api/projects/3/webhook?token=tok-123`,
    )
    expect(toast.success).toHaveBeenCalledWith('已复制')
  })

  it('剪贴板不可用：toast 错误（不静默）', async () => {
    clipboard.writeText.mockRejectedValueOnce(new Error('denied'))
    const ctl = mountCtl()
    ctl.toggleWebhook(row() as never, vi.fn())
    await settle()

    await ctl.copyWebhookUrl()
    expect(toast.error).toHaveBeenCalled()
  })
})
