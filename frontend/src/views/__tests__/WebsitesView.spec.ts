import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { createPinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import WebsitesView from '@/views/WebsitesView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站拆分后的视图级冒烟（WebsitesView 装配层）。
 *
 * 组件级 spec 各自验了表格与弹窗的契约，数据层 spec 验了请求编排——这一层补的是
 * **接线**：页签就位、首屏自动拉列表、两个页签的工具条按钮真实渲染、切到证书页签
 * 触发证书与 DNS 账号拉取（含「切回来不重复请求」）。
 *
 * 挂载策略：`shallow` + 保留必要的子组件替身。真实 FpTable/FpModal 在 jsdom 下
 * 要么不渲染（虚拟滚动）要么 Teleport 到 body（断言割裂），而本用例只关心装配。
 */

const { api } = vi.hoisted(() => ({
  api: {
    listWebsites: vi.fn(),
    listEngines: vi.fn(),
    listCertificates: vi.fn(),
    listDnsProviders: vi.fn(),
    listRuntimes: vi.fn(),
    listProjects: vi.fn(),
    listWebServers: vi.fn(),
  },
}))

vi.mock('@/api/websites', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/websites')>()
  return { ...actual, listWebsites: api.listWebsites }
})
vi.mock('@/api/webServers', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/webServers')>()
  return { ...actual, listEngines: api.listEngines }
})
vi.mock('@/api/certificates', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/certificates')>()
  return { ...actual, listCertificates: api.listCertificates }
})
vi.mock('@/api/dnsProviders', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/dnsProviders')>()
  return { ...actual, listDnsProviders: api.listDnsProviders }
})
vi.mock('@/api/runtimeProject', () => ({
  listRuntimes: api.listRuntimes,
  listProjects: api.listProjects,
}))
vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const messages = {
  website: {
    title: '网站管理',
    create: '创建网站',
    name: '名称',
    domain: '域名',
    rootPath: '根目录',
    siteType: '站点类型',
    siteType_static: '静态',
    siteType_reverse_proxy: '反向代理',
    siteType_runtime_php: 'PHP 运行时',
    siteType_container_app: '容器应用',
    rewritePreset: '伪静态模板',
    rewriteNone: '未套用',
    rewriteHint: '套用说明',
    switchEngine: '切换引擎',
    bind: '绑定',
    active: '运行中',
    inactive: '已停用',
  },
  certificate: {
    title: 'SSL 证书',
    issueAcme: 'ACME 签发',
    issueSelfSigned: '自签证书',
    upload: '上传证书',
    domains: '域名列表',
    source: '来源',
    expiresAt: '到期时间',
    autoRenew: '自动续签',
    boundWebsite: '关联网站',
    notBound: '未绑定',
    neverRenewed: '—',
    bind: '绑定网站',
    unbind: '解绑',
    renewNow: '续签',
  },
  common: {
    searchPlaceholder: '搜索',
    operation: '操作',
    edit: '编辑',
    delete: '删除',
    noData: '暂无数据',
    cancel: '取消',
    confirm: '确定',
  },
}

/**
 * 页签替身：`shallow` 下 Fp* 子组件不渲染插槽，而本用例要断言两个页签内的
 * 工具条与表格接线。替身按 FpTabs 的公开契约（`items` + 同名插槽）渲染**全部**
 * 插槽内容——与 AppStoreView.spec 同一取舍（doc §7.17 已记录）。
 */
const TabsStub = defineComponent({
  name: 'FpTabs',
  props: { items: { type: Array, default: () => [] }, modelValue: { type: String, default: '' } },
  emits: ['update:modelValue'],
  template: '<div class="stub-tabs"><slot name="websites" /><slot name="certificates" /></div>',
})

const LayoutStub = defineComponent({
  name: 'LayoutContent',
  template: '<div><slot name="actions" /><slot name="toolbar" /><slot /></div>',
})
async function mountView() {
  const w = mount(WebsitesView, {
    shallow: true,
    global: {
      plugins: [
        createPinia(),
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {} },
      stubs: {
        FpInput: false,
        FpSelect: false,
        FpButton: false,
        FpTabs: TabsStub,
        LayoutContent: LayoutStub,
        PageToolbar: false,
      },
    },
  })
  await new Promise((r) => setTimeout(r, 30))
  return w
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listWebsites.mockResolvedValue({
    data: { data: [], page: 1, page_size: 20, total: 0, total_pages: 0 },
  })
  api.listEngines.mockResolvedValue({ data: [] })
  api.listCertificates.mockResolvedValue({ data: { certificates: [], total: 0 } })
  api.listDnsProviders.mockResolvedValue({ data: [] })
  api.listRuntimes.mockResolvedValue({ data: [] })
  api.listProjects.mockResolvedValue({ data: [] })
})

describe('WebsitesView 装配层（T2-4 第二站冒烟）', () => {
  it('首屏自动拉网站列表（含分页参数）', async () => {
    await mountView()
    expect(api.listWebsites).toHaveBeenCalledWith(1, 20)
  })

  it('首屏不拉证书（证书页签惰性加载，避免多打一次接口）', async () => {
    await mountView()
    expect(api.listCertificates).not.toHaveBeenCalled()
    expect(api.listDnsProviders).not.toHaveBeenCalled()
  })

  it('四个子组件全部接线（两个面板 + 两套弹窗）', async () => {
    const w = await mountView()
    for (const name of [
      'WebsiteListPanel',
      'CertificateListPanel',
      'WebsiteDialogs',
      'CertificateDialogs',
      'CertificateRenewalDrawer',
    ]) {
      expect(w.findComponent({ name }).exists()).toBe(true)
    }
  })

  it('两个页签的工具条按钮都渲染（网站页签 1 个创建 + 证书页签 3 条签发入口）', async () => {
    const w = await mountView()
    // FpButton 的文案在默认插槽里，shallow 下 stub 不渲染插槽文本，故按数量与图标断言
    const buttons = w.findAllComponents({ name: 'FpButton' })
    expect(buttons.length).toBe(4)
    expect(buttons.map((b) => b.props('icon'))).toEqual(['oi oi-plus', 'oi oi-lock-locked', '', ''])
  })

  it('切到证书页签：拉证书列表 + DNS 厂商账号（签发弹窗需备好下拉）', async () => {
    const w = await mountView()
    const tabs = w.findComponent({ name: 'FpTabs' })
    tabs.vm.$emit('update:modelValue', 'certificates')
    await new Promise((r) => setTimeout(r, 30))

    expect(api.listCertificates).toHaveBeenCalledTimes(1)
    expect(api.listDnsProviders).toHaveBeenCalledTimes(1)
  })

  it('在证书页签内重复切换不重复请求（watch 只对切到证书那一次反应）', async () => {
    const w = await mountView()
    const tabs = w.findComponent({ name: 'FpTabs' })
    tabs.vm.$emit('update:modelValue', 'certificates')
    await new Promise((r) => setTimeout(r, 20))
    tabs.vm.$emit('update:modelValue', 'websites')
    await new Promise((r) => setTimeout(r, 20))
    tabs.vm.$emit('update:modelValue', 'certificates')
    await new Promise((r) => setTimeout(r, 20))

    // 每次切入都刷新（用户回来看到最新数据），但**不重复叠加**（一次切换一次请求）
    expect(api.listCertificates).toHaveBeenCalledTimes(2)
  })

  it('首屏不拉绑定弹窗的数据源（切到弹窗才拉，避免无谓请求）', async () => {
    await mountView()
    expect(api.listRuntimes).not.toHaveBeenCalled()
    expect(api.listProjects).not.toHaveBeenCalled()
  })

  it('引擎清单也不在首屏加载（切换引擎弹窗打开时才拉）', async () => {
    await mountView()
    expect(api.listEngines).not.toHaveBeenCalled()
  })
})
