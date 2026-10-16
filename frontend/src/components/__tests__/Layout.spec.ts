import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, nextTick } from 'vue'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import Layout from '../Layout.vue'
import { flushErrors } from '@/utils/monitor'

/**
 * R-2 集成回归（Doc/23 §2.5）：错误边界下沉到 Layout 的内容区。
 *
 * 验收点：**页面组件崩溃时，侧栏/顶栏/页脚骨架必须保留**——用户仍可切菜单自救。
 * 历史实现把 onErrorCaptured 挂在 App 并整棵替换 router-view，页面一崩就只剩一块
 * （且因文案/渲染缺陷而空白的）面板，用户失去全部自救入口。
 *
 * 说明：本条只关注「边界位置」，组件内部细节见 ui/__tests__/FpErrorBoundary.spec.ts。
 * 布局子组件经 `global.stubs` 打桩（避免真实组件的网络请求与 store 依赖），
 * 断言「桩仍在渲染树中」即等价于「导航骨架未被错误边界连带卸载」。
 */

/**
 * 测试页面组件统一经 `makePage` 工厂构造（单处 defineComponent 调用），
 * 避免 spec 内堆叠组件定义触发 vue/one-component-per-file。
 */
function makePage(name: string, setup: () => unknown) {
  return defineComponent({ name, setup: setup as () => never })
}

/** 崩溃页：setup 即抛错 */
const boomPage = () =>
  makePage('BoomPage', () => {
    throw new Error('page-crash')
  })

/** 正常页 */
const okPage = () => makePage('OkPage', () => () => h('div', { class: 'page-body' }, '页面内容'))

/**
 * 布局骨架组件名（与 Layout.vue 中的注册名一致）。
 * UX-1 后不再有 AppTabs（多页签栏已移除）；OnboardingTour 为懒加载组件，
 * 在此用 vi.mock 替换为空组件，避免测试中触发真实的锚点测量与 localStorage 读写。
 */
const SKELETON = ['AppSidebar', 'AppHeader', 'AppFooter'] as const

// 懒加载组件的 mock 工厂在首次 import() 时才执行（挂载阶段），此时 makePage 已初始化
vi.mock('@/components/onboarding/OnboardingTour.vue', () => ({
  // `__esModule` 让 defineAsyncComponent 识别为 ES 模块并取 default（vitest 的严格 mock
  // 代理对未声明的键会抛错，缺了它 Vue 会把整个模块命名空间当组件用）
  __esModule: true,
  default: makePage('OnboardingTour', () => () => null),
}))
const asStubs = (keep: readonly string[]) =>
  Object.fromEntries(SKELETON.map((n) => [n, !keep.includes(n)]))

/** 挂载 Layout（真实 router-view + store + i18n；布局子组件打桩） */
async function mountLayout(path: string) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', redirect: '/ok' },
      { path: '/ok', component: okPage() },
      { path: '/boom', component: boomPage() },
    ],
  })
  await router.push(path)
  await router.isReady()
  const i18n = createI18n({
    legacy: false,
    locale: 'zh-CN',
    messages: { 'zh-CN': { nav: { dashboard: '仪表盘' }, common: { retry: '重试' } } },
  })
  const w = mount(Layout, {
    global: {
      plugins: [router, createPinia(), i18n],
      stubs: asStubs([]),
    },
  })
  await nextTick()
  await nextTick()
  return { w, router }
}

describe('Layout 错误边界下沉（R-2）', () => {
  beforeAll(() => {
    // jsdom 未实现 matchMedia：Layout 用它做移动端断点探测，需最小 stub
    if (!window.matchMedia) {
      Object.defineProperty(window, 'matchMedia', {
        configurable: true,
        writable: true,
        value: (query: string) => ({
          matches: false,
          media: query,
          onchange: null,
          addEventListener: () => {},
          removeEventListener: () => {},
          addListener: () => {},
          removeListener: () => {},
          dispatchEvent: () => false,
        }),
      })
    }
  })
  beforeEach(() => {
    flushErrors()
    setActivePinia(createPinia())
    vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('正常页面：内容渲染且无错误面板', async () => {
    const { w } = await mountLayout('/ok')
    expect(w.find('.page-body').exists()).toBe(true)
    expect(w.find('.fp-eb').exists()).toBe(false)
  })

  it('UX-1/UX-2：不渲染多页签栏，但挂载新手引导（懒加载）', async () => {
    const { w } = await mountLayout('/ok')
    expect(w.findComponent({ name: 'AppTabs' }).exists()).toBe(false)
    // 异步组件需再等一轮微任务才会解析
    await new Promise((r) => setTimeout(r, 0))
    await nextTick()
    expect(w.findComponent({ name: 'OnboardingTour' }).exists()).toBe(true)
  })

  it('【关键回归】页面崩溃：错误面板出现在内容区，侧栏/顶栏/页签骨架保留', async () => {
    const { w } = await mountLayout('/boom')
    // 页面被错误边界接管
    expect(w.find('.fp-eb').exists()).toBe(true)
    expect(w.find('.page-body').exists()).toBe(false)
    // 导航骨架必须仍在——这是 R-2 的核心价值（用户可切菜单自救）
    expect(w.findComponent({ name: 'AppSidebar' }).exists()).toBe(true)
    expect(w.findComponent({ name: 'AppHeader' }).exists()).toBe(true)
    expect(w.findComponent({ name: 'AppFooter' }).exists()).toBe(true)
    // UX-1：多页签栏已移除，布局中不应再出现页签条
    expect(w.find('.app-tabs').exists()).toBe(false)
  })

  it('崩溃后切换到正常页面：错误面板随之消失', async () => {
    const { w, router } = await mountLayout('/boom')
    expect(w.find('.fp-eb').exists()).toBe(true)

    await router.push('/ok')
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(false)
    expect(w.find('.page-body').exists()).toBe(true)
  })
})
