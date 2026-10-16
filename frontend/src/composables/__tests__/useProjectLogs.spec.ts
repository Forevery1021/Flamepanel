import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站的日志抽屉数据层单测（`ProjectView` → `useProjectLogs`）。
 *
 * 日志之所以要单列一层：列表里**每一行都有「日志」按钮**，若跟着列表一起拉，
 * 进页面就会对每个项目各打一次 `/logs`（后端 tail 200 行，代价不低）。
 * 故本 spec 的重点是惰性与「换项目不留残影」这两条语义。
 */

const { api } = vi.hoisted(() => ({ api: { getProjectLogs: vi.fn() } }))

vi.mock('@/api/projects', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/projects')>()
  return { ...actual, getProjectLogs: api.getProjectLogs }
})
vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

import { useProjectLogs } from '@/composables/useProjectLogs'

const messages = { common: { failed: '操作失败', retry: '重试' } }

const row = (over: Record<string, unknown> = {}) => ({ id: 3, name: 'blog', ...over })

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useProjectLogs() }
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
    },
  })
  return (w.vm as unknown as { ctl: ReturnType<typeof useProjectLogs> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  api.getProjectLogs.mockResolvedValue({
    data: [
      { timestamp: '2026-09-15T10:00:00Z', message: 'started' },
      { timestamp: null, message: 'warn: slow' },
    ],
  })
})

describe('useProjectLogs', () => {
  it('【关键】挂载不拉日志（惰性：enabled: false）', async () => {
    mountCtl()
    await settle()
    expect(api.getProjectLogs).not.toHaveBeenCalled()
  })

  it('open(row)：按该项目 id 拉日志，文本已按行拼好', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    expect(api.getProjectLogs).toHaveBeenCalledWith(3)
    expect(ctl.project.value?.id).toBe(3)
    expect(ctl.text.value).toBe('[2026-09-15T10:00:00Z] started\nwarn: slow')
  })

  it('reset()：清空目标（下次打开是新项目，不残留旧日志）', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.reset()
    expect(ctl.project.value).toBeNull()
  })

  it('【关键】换项目重开：第二次请求打到新 id', async () => {
    const ctl = mountCtl()
    ctl.open(row({ id: 3 }) as never)
    await settle()
    ctl.open(row({ id: 8, name: 'api' }) as never)
    await settle()

    expect(api.getProjectLogs).toHaveBeenLastCalledWith(8)
  })

  it('失败：error 有文案（三态面板据此显示错误态），lines 保持空', async () => {
    api.getProjectLogs.mockRejectedValue(new Error('项目不存在'))
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    expect(ctl.error.value).toBeTruthy()
    expect(ctl.lines.value).toEqual([])
  })
})
