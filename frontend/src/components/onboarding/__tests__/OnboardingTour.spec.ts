import { describe, it, expect, vi, beforeEach, beforeAll } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import OnboardingTour from '@/components/onboarding/OnboardingTour.vue'
import { useOnboardingStore, ONBOARDING_STEPS } from '@/stores/onboarding'
import { useAuthStore } from '@/stores/auth'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * UX-2 新手引导组件：只测「不依赖真实几何」的行为——
 * 首次登录自动弹出、跳过按钮、Esc、锚点缺失时自动跳步（不会卡死）。
 * 聚光灯坐标由 getBoundingClientRect 决定，jsdom 全为 0，不在此断言。
 */

const messages = {
  onboarding: {
    ariaLabel: '新手引导',
    replay: '重新播放引导',
    skip: '跳过',
    prev: '上一步',
    next: '下一步',
    start: '开始导览',
    finish: '开始使用',
    steps: Object.fromEntries(
      ONBOARDING_STEPS.map((s) => [s.id, { title: `T-${s.id}`, body: `B-${s.id}` }]),
    ),
  },
}

async function mountTour(opts: { loggedIn?: boolean; anchors?: string[] } = {}) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const auth = useAuthStore()
  if (opts.loggedIn !== false) {
    auth.token = 'tok'
    auth.username = 'admin'
  }
  for (const a of opts.anchors ?? []) {
    const el = document.createElement('div')
    el.setAttribute('data-tour', a)
    document.body.appendChild(el)
  }
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
      { path: '/dashboard', component: { template: '<div />' } },
    ],
  })
  await router.push('/dashboard')
  await router.isReady()
  const w = mount(OnboardingTour, {
    attachTo: document.body,
    global: { plugins: [pinia, router, createTestI18n(messages), openVuePlugin, ...services] },
  })
  await flushPromises()
  await nextTick()
  return { w, store: useOnboardingStore(), router }
}

beforeAll(() => {
  // jsdom 无 requestAnimationFrame 节流语义差异：直接下一宏任务执行
  vi.stubGlobal(
    'requestAnimationFrame',
    (cb: FrameRequestCallback) => setTimeout(() => cb(0), 0) as unknown as number,
  )
  vi.stubGlobal('cancelAnimationFrame', (id: number) => clearTimeout(id))
})

beforeEach(() => {
  localStorage.clear()
  document.body.innerHTML = ''
})

describe('OnboardingTour', () => {
  it('已登录且未看过 → 自动弹出欢迎卡（含跳过按钮）', async () => {
    const { w, store } = await mountTour()
    expect(store.active).toBe(true)
    const card = document.body.querySelector('.tour-card')
    expect(card).not.toBeNull()
    expect(card!.textContent).toContain('T-welcome')
    expect(document.body.querySelector('.tour-skip')).not.toBeNull()
    w.unmount()
  })

  it('未登录不弹出', async () => {
    const { store } = await mountTour({ loggedIn: false })
    expect(store.active).toBe(false)
  })

  it('已完成过的用户不再自动弹出', async () => {
    setActivePinia(createPinia())
    useOnboardingStore().autoStart('admin')
    useOnboardingStore().finish()
    const { store } = await mountTour()
    expect(store.active).toBe(false)
  })

  it('点击「跳过」→ 关闭并持久化', async () => {
    const { store } = await mountTour()
    ;(document.body.querySelector('.tour-skip') as HTMLButtonElement).click()
    await nextTick()
    expect(store.active).toBe(false)
    expect(store.isDone('admin')).toBe(true)
  })

  it('Esc = 跳过', async () => {
    const { store } = await mountTour()
    const root = document.body.querySelector('.tour') as HTMLElement
    root.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()
    expect(store.active).toBe(false)
  })

  it('锚点缺失的步骤自动跳过，直到无锚点的结束页（不会卡死）', async () => {
    const { store } = await mountTour({ anchors: [] })
    store.next() // 离开欢迎页 → 第 1 步 sidebar（无锚点）
    // 逐步自动跳过：每步等待一次宏任务 + 微任务
    for (let i = 0; i < ONBOARDING_STEPS.length; i++) {
      await new Promise((r) => setTimeout(r, 0))
      await flushPromises()
    }
    expect(store.active).toBe(true)
    expect(store.current?.id).toBe('done')
  })

  it('锚点存在时停在该步并渲染标题', async () => {
    const { store } = await mountTour({ anchors: ['sidebar'] })
    store.next()
    await new Promise((r) => setTimeout(r, 0))
    await flushPromises()
    await nextTick()
    expect(store.current?.id).toBe('sidebar')
    expect(document.body.querySelector('.tour-card')!.textContent).toContain('T-sidebar')
  })
})
