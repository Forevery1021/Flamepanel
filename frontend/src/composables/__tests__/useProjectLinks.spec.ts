import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站的联链抽屉数据层单测（`ProjectView` → `useProjectLinks`）。
 *
 * 本族有两个**原实现就带着的坑**，本次拆分原样保留并逐条用用例锁死，
 * 以免将来「顺手修正」时无从判断哪些是契约、哪些是缺陷：
 *
 * ① 两个查询共用同一个 queryKey（`projects.links(id)`）——React Query 语义下
 *    同 key 即同缓存，后发的那条被去重命中（实测 linkable 只打一次、且可能
 *    拿到的是联链数据）。这是**既有缺陷**，已记入 PR「后续建议」；
 * ② 打开抽屉时清空表单（否则上一次填的 `env_name` 会被静默复用）。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listProjectLinks: vi.fn(),
    listLinkableDatabases: vi.fn(),
    linkProjectDatabase: vi.fn(),
    unlinkProjectDatabase: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; message: string } | null },
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

import { useProjectLinks } from '@/composables/useProjectLinks'

const messages = {
  project: {
    linkSuccess: '关联成功，启动项目后生效',
    linkUnlink: '解除关联',
    linkUnlinkConfirm: '确定解除环境变量 {env} 的数据库联链吗？',
  },
  common: { success: '操作成功', failed: '操作失败' },
}

const row = (over: Record<string, unknown> = {}) => ({ id: 3, name: 'blog', ...over })

const link = (over: Record<string, unknown> = {}) => ({
  id: 11,
  project_id: 3,
  db_instance_id: 5,
  db_instance_name: 'mysql-main',
  db_type: 'mysql',
  env_name: 'DATABASE_URL',
  port: 3306,
  ...over,
})

const linkable = (over: Record<string, unknown> = {}) => ({
  id: 5,
  name: 'mysql-main',
  db_type: 'mysql',
  port: 3306,
  status: 'running',
  default_env_name: 'DATABASE_URL',
  ...over,
})

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useProjectLinks() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useProjectLinks> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 25))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listProjectLinks.mockResolvedValue({ data: [link()] })
  api.listLinkableDatabases.mockResolvedValue({ data: [linkable()] })
  api.linkProjectDatabase.mockResolvedValue({ data: link() })
  api.unlinkProjectDatabase.mockResolvedValue({ data: { ok: true } })
})

describe('useProjectLinks · 打开与惰性', () => {
  it('挂载不拉数据（两条查询都 enabled: false）', async () => {
    mountCtl()
    await settle()
    expect(api.listProjectLinks).not.toHaveBeenCalled()
    expect(api.listLinkableDatabases).not.toHaveBeenCalled()
  })

  it('open(row)：记目标并按 id 发请求，已关联清单进 links', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    expect(api.listProjectLinks).toHaveBeenCalledWith(3)
    expect(ctl.links.value).toHaveLength(1)
    expect(ctl.project.value?.id).toBe(3)
  })

  it('【关键】open 清空表单：上一次填的 env_name 不被静默复用', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.form.value = { db_instance_id: 5, env_name: 'STALE_ENV' }
    ctl.open(row({ id: 8 }) as never)

    expect(ctl.form.value.db_instance_id).toBeUndefined()
    expect(ctl.form.value.env_name).toBe('')
  })

  it('reset()：清空目标', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()
    ctl.reset()
    expect(ctl.project.value).toBeNull()
  })
})

describe('useProjectLinks · 关联与解除', () => {
  it('关联：db_name 显式 null（后端按项目名推导业务库名），成功后清空表单并刷新', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()
    const before = api.listProjectLinks.mock.calls.length

    ctl.form.value = { db_instance_id: 5, env_name: 'DATABASE_URL' }
    await ctl.link()

    expect(api.linkProjectDatabase).toHaveBeenCalledWith(3, {
      db_instance_id: 5,
      env_name: 'DATABASE_URL',
      db_name: null,
    })
    expect(ctl.form.value.env_name).toBe('')
    expect(api.listProjectLinks.mock.calls.length).toBeGreaterThan(before)
  })

  it('env_name 留空：上报 null（空串会被当成名为空的环境变量）', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.form.value = { db_instance_id: 5, env_name: '' }
    await ctl.link()
    expect(api.linkProjectDatabase).toHaveBeenCalledWith(
      3,
      expect.objectContaining({ env_name: null }),
    )
  })

  it('未选实例：本地拦下，不发请求', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.form.value = { db_instance_id: undefined, env_name: 'X' }
    await ctl.link()
    expect(api.linkProjectDatabase).not.toHaveBeenCalled()
  })

  it('关联失败：toast 错误且 saving 复位（按钮不卡死）', async () => {
    api.linkProjectDatabase.mockRejectedValue(new Error('实例未运行'))
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.form.value = { db_instance_id: 5, env_name: '' }
    await ctl.link()

    expect(toast.error).toHaveBeenCalled()
    expect(ctl.saving.value).toBe(false)
  })

  it('【关键】解除：确认文案带环境变量名（同实例可能链了多个变量）', async () => {
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.unlink(link({ env_name: 'REDIS_URL' }) as never)
    expect(confirmState.last?.message).toBe('确定解除环境变量 REDIS_URL 的数据库联链吗？')
    expect(api.unlinkProjectDatabase).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.unlinkProjectDatabase).toHaveBeenCalledWith(3, 11)
    expect(toast.success).toHaveBeenCalled()
  })

  it('【关键】解除的锁是 per-link：请求期间只锁该条', async () => {
    let release: () => void = () => {}
    api.unlinkProjectDatabase.mockImplementation(
      () => new Promise((resolve) => (release = () => resolve({ data: { ok: true } }))),
    )
    const ctl = mountCtl()
    ctl.open(row() as never)
    await settle()

    ctl.unlink(link({ id: 21 }) as never)
    confirmState.last?.accept?.()
    await settle()
    expect(ctl.actingId.value).toBe('unlink-21')

    release()
    await settle()
    expect(ctl.actingId.value).toBe('')
  })
})
