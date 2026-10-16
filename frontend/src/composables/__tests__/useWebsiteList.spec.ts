import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站的数据层单测（`WebsitesView` → `useWebsiteList`）。
 *
 * 这层逻辑原先是「1181 行巨型视图内的内联函数」——既在 `src/views/**`（覆盖率
 * 排除区），又没人能脱离整页挂载去验证它。拆出后本 spec 直接看住四件事：
 * ① 分页查询的请求参数与 total 回写；② 搜索 + 状态过滤（当前页客户端过滤）；
 * ③ 新建/编辑的请求体组装（含 `proxy_enabled` 决定 `proxy_pass` 的既有语义）；
 * ④ 删除与引擎切换的确认分支与错误容错。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listWebsites: vi.fn(),
    createWebsite: vi.fn(),
    updateWebsite: vi.fn(),
    deleteWebsite: vi.fn(),
    switchWebsiteEngine: vi.fn(),
    deployWebsite: vi.fn(),
    listEngines: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as { accept?: () => void; message: string } | null },
}))

vi.mock('@/api/websites', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/websites')>()
  return { ...actual, ...api }
})
vi.mock('@/api/webServers', () => ({ listEngines: api.listEngines }))
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useWebsiteList, type WebsiteFormPayload } from '@/composables/useWebsiteList'

const messages = {
  website: {
    deleteConfirm: '确定要删除网站 {name} 吗？',
    deploySuccess: '已部署',
    deployPendingReload: '已写入，启动后生效',
    deployFailed: '部署失败',
  },
  common: { success: '成功', failed: '操作失败', confirmAction: '确定执行此操作吗？' },
}

const site = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'blog',
  domain: 'blog.example.com',
  root_path: '/www/blog',
  node_id: 1,
  status: 'active',
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: null,
  site_type: 'static',
  rewrite_preset: null,
  created_at: '2026-01-01 00:00:00',
  ...over,
})

const form = (over: Partial<WebsiteFormPayload> = {}): WebsiteFormPayload => ({
  name: 'blog',
  domain: 'blog.example.com',
  root_path: '/www/blog',
  node_id: 1,
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: '',
  site_type: 'static',
  ...over,
})

/** 宿主组件：数据层与 query client 同挂，用例直接读写 */
function makeHost() {
  return defineComponent({
    setup() {
      return { list: useWebsiteList() }
    },
    template: '<div/>',
  })
}

function mountList() {
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
  const list = (w.vm as unknown as { list: ReturnType<typeof useWebsiteList> }).list
  return { w, list }
}

/** 等待挂载期自动查询 resolve */
async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listWebsites.mockResolvedValue({
    data: { data: [site()], page: 1, page_size: 20, total: 1, total_pages: 1 },
  })
  api.createWebsite.mockResolvedValue({ data: 1 })
  api.updateWebsite.mockResolvedValue({ data: site() })
  api.deleteWebsite.mockResolvedValue({ data: null })
  api.switchWebsiteEngine.mockResolvedValue({ data: site({ engine: 'apache' }) })
  api.listEngines.mockResolvedValue({ data: [{ name: 'nginx' }, { name: 'apache' }] })
})

describe('useWebsiteList · 列表与分页', () => {
  it('挂载即拉首页（参数含分页），并把 total 回写到分页状态', async () => {
    api.listWebsites.mockResolvedValue({
      data: { data: [site(), site({ id: 2 })], page: 1, page_size: 20, total: 42, total_pages: 3 },
    })
    const { list } = mountList()
    await settle()

    expect(api.listWebsites).toHaveBeenCalledWith(1, 20)
    expect(list.websites.value).toHaveLength(2)
    expect(list.total.value).toBe(42)
    expect(list.loading.value).toBe(false)
  })

  it('翻页：onFirst 换算页号后重新查询（FpPagination 的 first 为偏移量）', async () => {
    const { list } = mountList()
    await settle()

    list.onFirst(20)
    await settle()
    expect(api.listWebsites).toHaveBeenLastCalledWith(2, 20)
    expect(list.first.value).toBe(20)
  })

  /**
   * 注意：`useApiQuery` 显式传 `retry: undefined`，vue-query 会因此回落到**库默认**
   * 的 3 次重试（客户端 `defaultOptions` 的 `retry: false` 不生效）——这是该封装层
   * 的存量行为（已记入 PR「后续建议」），不是本次拆分引入。故本用例用假定时器把
   * 三次退避（1s+2s+4s）一次性推完，断言最终落到错误态；真实用户侧表现不变
   * （负载均衡/网络抖动下的自动重试本就是期望语义）。
   */
  it('请求失败：重试耗尽后 error 非空且 loading 复位（页面不卡死转圈）', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    try {
      api.listWebsites.mockRejectedValue(new Error('boom'))
      const { list } = mountList()
      await vi.advanceTimersByTimeAsync(30_000)

      expect(list.error.value).toBeTruthy()
      expect(list.loading.value).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('useWebsiteList · 搜索与状态过滤（当前页客户端过滤）', () => {
  it('关键字覆盖 name / domain / root_path 三个字段', async () => {
    api.listWebsites.mockResolvedValue({
      data: {
        data: [
          site({ id: 1, name: 'blog', domain: 'a.com', root_path: '/www/a' }),
          site({ id: 2, name: 'shop', domain: 'b.com', root_path: '/www/b' }),
        ],
        page: 1,
        page_size: 20,
        total: 2,
        total_pages: 1,
      },
    })
    const { list } = mountList()
    await settle()

    list.search.value = 'SHOP' // 大小写不敏感
    await settle()
    expect(list.filteredWebsites.value.map((s) => s.id)).toEqual([2])

    list.search.value = '/www/a'
    await settle()
    expect(list.filteredWebsites.value.map((s) => s.id)).toEqual([1])

    list.search.value = ''
    await settle()
    expect(list.filteredWebsites.value).toHaveLength(2)
  })

  it('状态过滤：inactive 只留停用站点，清空后恢复全量', async () => {
    api.listWebsites.mockResolvedValue({
      data: {
        data: [site({ id: 1, status: 'active' }), site({ id: 2, status: 'inactive' })],
        page: 1,
        page_size: 20,
        total: 2,
        total_pages: 1,
      },
    })
    const { list } = mountList()
    await settle()

    list.statusFilter.value = 'inactive'
    await settle()
    expect(list.filteredWebsites.value.map((s) => s.id)).toEqual([2])

    list.statusFilter.value = ''
    await settle()
    expect(list.filteredWebsites.value).toHaveLength(2)
  })

  it('搜索与状态过滤叠加（两个条件同时生效）', async () => {
    api.listWebsites.mockResolvedValue({
      data: {
        data: [
          site({ id: 1, name: 'blog-a', status: 'active' }),
          site({ id: 2, name: 'blog-b', status: 'inactive' }),
        ],
        page: 1,
        page_size: 20,
        total: 2,
        total_pages: 1,
      },
    })
    const { list } = mountList()
    await settle()

    list.search.value = 'blog'
    list.statusFilter.value = 'inactive'
    await settle()
    expect(list.filteredWebsites.value.map((s) => s.id)).toEqual([2])
  })
})

describe('useWebsiteList · 新建 / 编辑', () => {
  it('新建请求体：id=0、status=active、created_at 空串（与后端协议一致）', async () => {
    const { list } = mountList()
    await settle()

    const ok = await list.create(form({ name: 'new-site', domain: 'new.com' }))
    expect(ok).toBe(true)
    expect(api.createWebsite).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 0,
        name: 'new-site',
        domain: 'new.com',
        status: 'active',
        created_at: '',
      }),
    )
    expect(toast.success).toHaveBeenCalledWith('成功')
    // 成功后刷新列表
    expect(api.listWebsites.mock.calls.length).toBeGreaterThanOrEqual(2)
  })

  it('未开启反代：proxy_pass 上报 null（历史语义：关闭时清空代理地址）', async () => {
    const { list } = mountList()
    await settle()

    await list.create(form({ proxy_enabled: false, proxy_pass: 'http://127.0.0.1:8080' }))
    expect(api.createWebsite).toHaveBeenCalledWith(
      expect.objectContaining({ proxy_enabled: false, proxy_pass: null }),
    )
  })

  it('开启反代：proxy_pass 上报实际地址', async () => {
    const { list } = mountList()
    await settle()

    await list.create(form({ proxy_enabled: true, proxy_pass: 'http://127.0.0.1:8080' }))
    expect(api.createWebsite).toHaveBeenCalledWith(
      expect.objectContaining({ proxy_enabled: true, proxy_pass: 'http://127.0.0.1:8080' }),
    )
  })

  it('编辑：id 同时进路径与请求体（后端按 resource_version 做并发校验的前置契约）', async () => {
    const { list } = mountList()
    await settle()

    const ok = await list.update(7, form({ name: 'renamed' }))
    expect(ok).toBe(true)
    expect(api.updateWebsite).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ id: 7, name: 'renamed' }),
    )
  })

  it('新建失败：返回 false 且 toast 报错（调用方据此不关弹窗，用户可修正重试）', async () => {
    api.createWebsite.mockRejectedValue(new Error('boom'))
    const { list } = mountList()
    await settle()

    expect(await list.create(form())).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('编辑失败：返回 false 且 toast 报错（与新建同一容错语义）', async () => {
    api.updateWebsite.mockRejectedValue(new Error('boom'))
    const { list } = mountList()
    await settle()

    expect(await list.update(7, form())).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('提交期间 submitting 为 true，结束后复位（防重复提交）', async () => {
    let release: (v: unknown) => void = () => {}
    api.createWebsite.mockReturnValue(new Promise((r) => (release = r)))
    const { list } = mountList()
    await settle()

    const pending = list.create(form())
    await settle()
    expect(list.submitting.value).toBe(true)

    release({ data: 1 })
    await pending
    expect(list.submitting.value).toBe(false)
  })
})

describe('useWebsiteList · 删除（二次确认）', () => {
  it('先弹确认；确认后才发请求并刷新', async () => {
    const { list } = mountList()
    await settle()

    list.remove(site({ id: 9, name: 'old-site' }))
    expect(confirmState.last?.message).toContain('old-site')
    expect(api.deleteWebsite).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await settle()
    expect(api.deleteWebsite).toHaveBeenCalledWith(9)
  })

  it('删除失败：toast 报错，列表不崩', async () => {
    api.deleteWebsite.mockRejectedValue(new Error('boom'))
    const { list } = mountList()
    await settle()

    list.remove(site({ id: 9 }))
    confirmState.last?.accept?.()
    await settle()
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useWebsiteList · 引擎切换', () => {
  it('引擎清单懒加载：首次调用才请求，二次调用复用缓存', async () => {
    const { list } = mountList()
    await settle()

    await list.loadEngines()
    await list.loadEngines()
    expect(api.listEngines).toHaveBeenCalledTimes(1)
    expect(list.engines.value).toHaveLength(2)
  })

  it('引擎清单加载失败：toast 报错，不抛异常（弹窗仍可打开）', async () => {
    api.listEngines.mockRejectedValue(new Error('boom'))
    const { list } = mountList()
    await settle()

    await list.loadEngines()
    expect(toast.error).toHaveBeenCalled()
  })

  it('空引擎名不发请求（下拉未选时点确认不应产生无效调用）', async () => {
    const { list } = mountList()
    await settle()

    expect(await list.switchEngine(1, '')).toBe(false)
    expect(api.switchWebsiteEngine).not.toHaveBeenCalled()
  })

  it('切换成功：返回 true 并刷新列表（引擎变化影响 vhost 渲染）', async () => {
    const { list } = mountList()
    await settle()
    const before = api.listWebsites.mock.calls.length

    expect(await list.switchEngine(3, 'apache')).toBe(true)
    expect(api.switchWebsiteEngine).toHaveBeenCalledWith(3, 'apache')
    expect(api.listWebsites.mock.calls.length).toBe(before + 1)
  })

  it('切换失败：返回 false（调用方据此不关弹窗）', async () => {
    api.switchWebsiteEngine.mockRejectedValue(new Error('boom'))
    const { list } = mountList()
    await settle()

    expect(await list.switchEngine(3, 'apache')).toBe(false)
  })
})

describe('useWebsiteList · 部署配置', () => {
  it('部署成功且已 reload：成功提示，deployingId 复位', async () => {
    api.deployWebsite.mockResolvedValue({
      data: {
        engine: 'nginx',
        path: '/etc/nginx/flamepanel/sites/a.conf',
        enabled: true,
        reloaded: true,
      },
    })
    const { list } = mountList()
    await settle()

    const p = list.deploy(site({ id: 7 }) as never)
    expect(list.deployingId.value).toBe(7)
    expect(await p).toBe(true)
    expect(api.deployWebsite).toHaveBeenCalledWith(7)
    expect(toast.success).toHaveBeenCalledWith('已部署')
    expect(list.deployingId.value).toBeNull()
  })

  it('引擎未运行（reloaded=false）：提示启动后生效', async () => {
    api.deployWebsite.mockResolvedValue({
      data: { engine: 'nginx', path: '/x', enabled: true, reloaded: false },
    })
    const { list } = mountList()
    await settle()

    await list.deploy(site() as never)
    expect(toast.success).toHaveBeenCalledWith('已写入，启动后生效')
  })

  it('4xx 失败：带出后端原因（如 nginx -t 输出）', async () => {
    api.deployWebsite.mockRejectedValue(
      Object.assign(new Error('Bad request: nginx config test failed: line 3'), {
        status: 400,
        code: 'BAD_REQUEST',
      }),
    )
    const { list } = mountList()
    await settle()

    expect(await list.deploy(site() as never)).toBe(false)
    expect(toast.error).toHaveBeenCalledWith(
      '部署失败: Bad request: nginx config test failed: line 3',
    )
    expect(list.deployingId.value).toBeNull()
  })

  it('5xx 失败：只显示通用文案（服务端已脱敏）', async () => {
    api.deployWebsite.mockRejectedValue(
      Object.assign(new Error('internal server error'), { status: 500, code: 'INTERNAL' }),
    )
    const { list } = mountList()
    await settle()

    await list.deploy(site() as never)
    expect(toast.error).toHaveBeenCalledWith('部署失败')
  })
})
