import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useOnboardingStore, ONBOARDING_STEPS, ONBOARDING_VERSION } from '@/stores/onboarding'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * UX-2 新手引导状态机：首次自动触发 / 可跳过 / 完成与跳过都按用户名持久化 / 重放。
 * 组件层（聚光灯测量）依赖 DOM 几何，不在此覆盖；这里锁定的是「什么时候弹、弹几次」。
 */
beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
})

describe('useOnboardingStore · 触发策略', () => {
  it('首次进入自动开始，并从第 0 步（欢迎页）起', () => {
    const s = useOnboardingStore()
    expect(s.autoStart('admin')).toBe(true)
    expect(s.active).toBe(true)
    expect(s.index).toBe(0)
    expect(s.current?.id).toBe('welcome')
    expect(s.total).toBe(ONBOARDING_STEPS.length)
  })

  it('跳过后写入完成标记，同一用户再进入不再自动弹出', () => {
    const s = useOnboardingStore()
    s.autoStart('admin')
    s.skip()
    expect(s.active).toBe(false)
    const raw = localStorage.getItem(`${STORAGE_KEYS.onboarding}.admin`)
    expect(raw).toBeTruthy()
    expect(JSON.parse(raw!)).toMatchObject({ version: ONBOARDING_VERSION, outcome: 'skipped' })
    // 新的 store 实例（模拟刷新）
    setActivePinia(createPinia())
    expect(useOnboardingStore().autoStart('admin')).toBe(false)
  })

  it('完成标记按用户名隔离：admin 看过不影响 operator', () => {
    const s = useOnboardingStore()
    s.autoStart('admin')
    s.finish()
    expect(s.autoStart('operator')).toBe(true)
  })

  it('已完成的用户仍可手动重放（start 不看持久化标记）', () => {
    const s = useOnboardingStore()
    s.autoStart('admin')
    s.finish()
    s.start()
    expect(s.active).toBe(true)
    expect(s.index).toBe(0)
  })

  it('版本升级后重新触发（步骤大改时提升 ONBOARDING_VERSION）', () => {
    localStorage.setItem(
      `${STORAGE_KEYS.onboarding}.admin`,
      JSON.stringify({ version: ONBOARDING_VERSION - 1, outcome: 'completed' }),
    )
    expect(useOnboardingStore().autoStart('admin')).toBe(true)
  })

  it('损坏的持久化值视为未完成（不抛错）', () => {
    localStorage.setItem(`${STORAGE_KEYS.onboarding}.admin`, '{not json')
    expect(useOnboardingStore().autoStart('admin')).toBe(true)
  })
})

describe('useOnboardingStore · 步进', () => {
  it('next 到末步再 next = finish；prev 在首步无效', () => {
    const s = useOnboardingStore()
    s.autoStart('admin')
    s.prev()
    expect(s.index).toBe(0)
    for (let i = 0; i < ONBOARDING_STEPS.length - 1; i++) s.next()
    expect(s.isLast).toBe(true)
    s.next()
    expect(s.active).toBe(false)
    expect(s.isDone('admin')).toBe(true)
  })

  it('goTo 越界忽略；progress 为 (index+1)/total', () => {
    const s = useOnboardingStore()
    s.autoStart('admin')
    s.goTo(99)
    expect(s.index).toBe(0)
    s.goTo(2)
    expect(s.index).toBe(2)
    expect(s.progress).toBeCloseTo(3 / ONBOARDING_STEPS.length)
  })

  it('步骤定义：每步 id 唯一，带锚点的步骤都声明了位置', () => {
    const ids = ONBOARDING_STEPS.map((x) => x.id)
    expect(new Set(ids).size).toBe(ids.length)
    for (const step of ONBOARDING_STEPS) {
      if (step.anchor) expect(step.placement).toBeTruthy()
    }
  })
})
