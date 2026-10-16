import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * 新手操作向导状态（UX-2）。
 *
 * - **触发**：首次进入面板（该用户名下无完成/跳过记录）→ 自动开始；用户菜单「重新播放引导」可随时重放；
 * - **可跳过**：任一步都能「跳过」，跳过与完成都会写入完成标记（`flamepanel.onboarding.<username>`），
 *   之后不再自动弹出；
 * - **步骤定义在这里而非组件里**：步骤 = 「锚点选择器 + 文案 key + 建议位置」的纯数据，
 *   组件只负责测量锚点与渲染聚光灯；新增一步只加一条数据，且可被单测直接断言。
 *
 * 存储按用户名隔离：同一浏览器多个账号（admin / operator）各自看一次。
 */

export interface OnboardingStep {
  /** 唯一 id（也是 i18n key 后缀：onboarding.steps.<id>.title / .body） */
  id: string
  /** 锚点：`[data-tour="<anchor>"]`；空 = 居中卡片（欢迎/结束页） */
  anchor?: string
  /** 卡片相对锚点的位置偏好 */
  placement?: 'right' | 'bottom' | 'left' | 'top'
  /** 该步需要先跳转到的路由（如仪表盘卡片必须在 /dashboard 才存在） */
  route?: string
}

export const ONBOARDING_VERSION = 1

export const ONBOARDING_STEPS: readonly OnboardingStep[] = [
  { id: 'welcome' },
  { id: 'sidebar', anchor: 'sidebar', placement: 'right' },
  { id: 'stats', anchor: 'dashboard-stats', placement: 'bottom', route: '/dashboard' },
  { id: 'search', anchor: 'search', placement: 'bottom' },
  { id: 'status', anchor: 'status', placement: 'bottom' },
  { id: 'theme', anchor: 'theme', placement: 'bottom' },
  { id: 'settings', anchor: 'settings', placement: 'right' },
  { id: 'done' },
] as const

function storageKey(username: string) {
  return `${STORAGE_KEYS.onboarding}.${username || 'anonymous'}`
}

export const useOnboardingStore = defineStore('onboarding', () => {
  const active = ref(false)
  const index = ref(0)
  const username = ref('')

  const steps = computed(() => ONBOARDING_STEPS)
  const current = computed(() => (active.value ? (steps.value[index.value] ?? null) : null))
  const total = computed(() => steps.value.length)
  const isFirst = computed(() => index.value === 0)
  const isLast = computed(() => index.value === steps.value.length - 1)
  const progress = computed(() => (total.value ? (index.value + 1) / total.value : 0))

  /** 该用户是否已完成/跳过过（含版本：步骤大改时提升 ONBOARDING_VERSION 可再次触发） */
  function isDone(user: string): boolean {
    try {
      const raw = localStorage.getItem(storageKey(user))
      if (!raw) return false
      const parsed = JSON.parse(raw) as { version?: number }
      return (parsed.version ?? 0) >= ONBOARDING_VERSION
    } catch {
      return false
    }
  }

  function markDone(user: string, outcome: 'completed' | 'skipped') {
    try {
      localStorage.setItem(
        storageKey(user),
        JSON.stringify({ version: ONBOARDING_VERSION, outcome, at: new Date().toISOString() }),
      )
    } catch {
      // 隐私模式等写入失败：本会话内 active=false 已足够，不阻塞用户
    }
  }

  /** 首次进入自动触发（幂等：已完成的用户不再弹出） */
  function autoStart(user: string): boolean {
    username.value = user
    if (isDone(user)) return false
    start()
    return true
  }

  function start() {
    index.value = 0
    active.value = true
  }

  function next() {
    if (isLast.value) {
      finish()
      return
    }
    index.value += 1
  }

  function prev() {
    if (!isFirst.value) index.value -= 1
  }

  function goTo(i: number) {
    if (i >= 0 && i < steps.value.length) index.value = i
  }

  function skip() {
    active.value = false
    markDone(username.value, 'skipped')
  }

  function finish() {
    active.value = false
    markDone(username.value, 'completed')
  }

  /** 测试/登出用：清空该用户的完成标记 */
  function reset(user: string) {
    try {
      localStorage.removeItem(storageKey(user))
    } catch {
      // ignore
    }
  }

  return {
    active,
    index,
    username,
    steps,
    current,
    total,
    isFirst,
    isLast,
    progress,
    isDone,
    autoStart,
    start,
    next,
    prev,
    goTo,
    skip,
    finish,
    reset,
  }
})
