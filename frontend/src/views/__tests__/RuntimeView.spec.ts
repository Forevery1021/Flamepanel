import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import RuntimeView from '@/views/RuntimeView.vue'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import { permission } from '@/directives/permission'
import * as runtimesApi from '@/api/runtimes'

/**
 * FpCard 默认插槽缺陷（T1-1 发现）的用户可见影响面回归。
 *
 * 历史行为：FpCard 的 `<slot />` 落进底层 `openvue/card` 的默认插槽，而后者只渲染
 * 命名插槽 `content` —— 卡片主体**整体不可见且无任何报错**。RuntimeView 是直接写
 * 默认插槽的唯一消费方：安装路径与「安装 / 设为默认 / 卸载」按钮全部丢失。
 *
 * 本 spec 从视图层贯穿「RuntimeView → FpCard → p-card-content → 卡片主体」整条链路，
 * 断言卡片正文与操作按钮真实落到内容区——组件级断言（FpCard.spec）无法覆盖
 * 「消费方渲染结果」这一层。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

vi.mock('@/api/runtimes', () => ({
  RUNTIME_KINDS: ['php', 'java', 'node', 'python', 'go', 'tomcat'],
  availableRuntimes: vi.fn().mockResolvedValue({
    data: [{ version: '8.2', installed: true, is_default: true, runtime_id: 7 }],
  }),
  listRuntimes: vi.fn().mockResolvedValue({
    data: [
      {
        id: 7,
        kind: 'php',
        version: '8.2',
        status: 'installed',
        install_path: '/opt/flame/runtimes/php/8.2',
        is_default: true,
        created_at: '',
        updated_at: '',
      },
    ],
  }),
  installRuntime: vi.fn(),
  uninstallRuntime: vi.fn(),
  setDefaultRuntime: vi.fn(),
}))

const messages = {
  runtime: {
    title: '运行环境',
    default: '默认',
    installed: '已安装',
    notInstalled: '未安装',
    kindLabel: '语言',
    installPath: '安装路径',
    setDefault: '设为默认',
    uninstall: '卸载',
    resolvedVersion: '实际版本',
    node: { withPm2: '同时安装 pm2', hint: 'Node 提示' },
    python: { hint: 'Python 提示' },
    go: { hint: 'Go 提示' },
    java: { hint: 'Java 提示' },
    kinds: {
      php: 'PHP',
      java: 'Java',
      node: 'Node.js',
      python: 'Python',
      go: 'Go',
      tomcat: 'Tomcat',
    },
  },
  common: { install: '安装' },
}

async function mountRuntime() {
  const pinia = createPinia()
  setActivePinia(pinia)
  // 权限指令读 store 角色：admin 让操作按钮全部可见（断言按钮真实渲染）
  useAuthStore().role = 'admin'

  const w = mount(RuntimeView, {
    global: {
      plugins: [
        pinia,
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      // 走真实 v-permission（admin 角色下全部放行）：按钮显隐与生产一致
      directives: { permission },
    },
  })
  // 等两个查询 resolve + 渲染刷新
  await new Promise((resolve) => setTimeout(resolve, 30))
  await nextTick()
  return w
}

describe('FpCard 插槽修复的视图级回归（RuntimeView 卡片主体可见）', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => localStorage.clear())

  it('卡片主体渲染进内容区（历史缺陷：整体不可见）', async () => {
    const w = await mountRuntime()
    const content = w.find('.p-card-content')
    expect(content.exists()).toBe(true)
    const body = content.find('.runtime-card__body')
    expect(body.exists()).toBe(true)
    // 元信息行（语言 / 安装路径）可见
    expect(body.text()).toContain('语言：PHP')
    expect(body.text()).toContain('/opt/flame/runtimes/php/8.2')
  })

  it('操作按钮随主体一起渲染在内容区（非标题栏/页面外）', async () => {
    const w = await mountRuntime()
    const body = w.find('.p-card-content .runtime-card__body')
    const buttons = body.findAll('.fp-btn')
    // 已安装 + 默认版本：安装与卸载按钮随主体渲染，设为默认按条件隐藏——
    // 历史缺陷下这些按钮**一个都不存在**（主体整体不可见）。
    expect(buttons.length).toBeGreaterThan(0)
    const texts = buttons.map((b) => b.text())
    expect(texts).toContain('安装')
    const uninstall = buttons.find((b) => b.text() === '卸载')!
    expect(uninstall.isVisible()).toBe(true)
    // 按钮确实落在卡片内容区，且不在标题栏
    expect(w.find('.fp-card-title .fp-btn').exists()).toBe(false)
  })

  it('【关键回归】已安装版本的「安装」按钮保持自身 disabled（v-permission 不得抹除）', async () => {
    // C 系列缺陷：`v-permission` 的 restore() 无条件 removeAttribute('disabled')，
    // 放行分支把**元素自身**的禁用态一并撤销。RuntimeView 的「安装」按钮
    // `:disabled="item.installed"`（当前 mock 中已安装 → 应为 true）在 admin 下
    // 渲染成可点击，用户能对已安装版本重复发起安装。
    // 断言走得比「属性存在」更深一层：直接读 DOM 的 disabled **属性态**（真实可交互性）。
    const w = await mountRuntime()
    const body = w.find('.p-card-content .runtime-card__body')
    const install = body.findAll('.fp-btn').find((b) => b.text() === '安装')
    expect(install).toBeTruthy()
    expect((install!.element as HTMLButtonElement).disabled).toBe(true)
    expect(install!.attributes('disabled')).toBeDefined()
  })

  it('正文不落在标题栏内（命名插槽转发未串位）', async () => {
    const w = await mountRuntime()
    expect(w.find('.fp-card-title .runtime-card__body').exists()).toBe(false)
    expect(w.find('.p-card-content .runtime-card__body').exists()).toBe(true)
  })
})

describe('RuntimeView · S-11 Node 运行时', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.mocked(runtimesApi.availableRuntimes).mockImplementation((async (kind: string) =>
      kind === 'node'
        ? {
            data: [
              { version: '20', installed: true, is_default: false, runtime_id: 9 },
              { version: '22', installed: false, is_default: false },
            ],
          }
        : { data: [] }) as never)
    vi.mocked(runtimesApi.listRuntimes).mockResolvedValue({
      data: [
        {
          id: 9,
          kind: 'node',
          version: '20',
          status: 'installed',
          install_path: '/opt/flamepanel/runtimes/node/20',
          is_default: false,
          metadata_json: '{"version":"20.18.1","pm2":true}',
          created_at: '',
          updated_at: '',
        },
      ],
    } as never)
    vi.mocked(runtimesApi.installRuntime).mockResolvedValue({ data: {} } as never)
  })

  async function mountNode() {
    const w = await mountRuntime()
    await w.findComponent({ name: 'FpSelectButton' }).vm.$emit('update:modelValue', 'node')
    await new Promise((resolve) => setTimeout(resolve, 30))
    await nextTick()
    return w
  }

  it('Node 页签显示 pm2 开关与提示；已安装版本展示解析后的实际版本与 pm2 标记', async () => {
    const w = await mountNode()
    expect(w.text()).toContain('同时安装 pm2')
    expect(w.text()).toContain('Node 提示')
    expect(w.text()).toContain('实际版本：v20.18.1')
    expect(w.find('.runtime-pm2-tag').exists()).toBe(true)
  })

  it('勾选 pm2 后安装请求带 pm2: true；未勾选不带该字段', async () => {
    const w = await mountNode()
    const installBtn = () =>
      w
        .findAll('.fp-btn')
        .filter((b) => b.text() === '安装' && !(b.element as HTMLButtonElement).disabled)[0]!
    await installBtn().trigger('click')
    expect(runtimesApi.installRuntime).toHaveBeenLastCalledWith({ kind: 'node', version: '22' })

    await w.findComponent({ name: 'FpSwitch' }).vm.$emit('update:modelValue', true)
    await nextTick()
    await installBtn().trigger('click')
    expect(runtimesApi.installRuntime).toHaveBeenLastCalledWith({
      kind: 'node',
      version: '22',
      pm2: true,
    })
  })
})

describe('RuntimeView · M-C 收尾 Python / Go / Java', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.mocked(runtimesApi.availableRuntimes).mockImplementation((async (kind: string) =>
      kind === 'java'
        ? { data: [{ version: '21', installed: true, is_default: true, runtime_id: 11 }] }
        : { data: [] }) as never)
    vi.mocked(runtimesApi.listRuntimes).mockResolvedValue({
      data: [
        {
          id: 11,
          kind: 'java',
          version: '21',
          status: 'installed',
          install_path: '/opt/flamepanel/runtimes/java/21',
          is_default: true,
          metadata_json: '{"version":"21.0.4+7","vendor":"Eclipse Temurin"}',
          created_at: '',
          updated_at: '',
        },
      ],
    } as never)
  })

  async function mountKind(kind: string) {
    const w = await mountRuntime()
    await w.findComponent({ name: 'FpSelectButton' }).vm.$emit('update:modelValue', kind)
    await new Promise((resolve) => setTimeout(resolve, 30))
    await nextTick()
    return w
  }

  it('Java 页签显示安装说明、实际版本与 JDK 发行商，且没有 pm2 开关', async () => {
    const w = await mountKind('java')
    expect(w.text()).toContain('Java 提示')
    expect(w.text()).toContain('实际版本：v21.0.4+7')
    expect(w.text()).toContain('Eclipse Temurin')
    expect(w.text()).not.toContain('同时安装 pm2')
  })

  it('Python / Go 页签各自显示说明；PHP 页签没有说明', async () => {
    expect((await mountKind('python')).text()).toContain('Python 提示')
    expect((await mountKind('go')).text()).toContain('Go 提示')
    const php = await mountKind('php')
    expect(php.find('.runtime-hint').exists()).toBe(false)
  })
})
