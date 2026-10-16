<template>
  <Teleport to="body">
    <Transition name="tour">
      <div
        v-if="store.active && step"
        class="tour"
        role="dialog"
        :aria-label="t('onboarding.ariaLabel')"
        @keydown="onKeydown"
      >
        <!-- 聚光灯遮罩：四块矩形围出锚点洞（不遮住目标本身，用户可看清高亮区域） -->
        <svg class="tour-mask" aria-hidden="true">
          <defs>
            <mask :id="maskId">
              <rect x="0" y="0" width="100%" height="100%" fill="white" />
              <rect
                v-if="spot"
                :x="spot.x"
                :y="spot.y"
                :width="spot.w"
                :height="spot.h"
                :rx="spot.r"
                fill="black"
              />
            </mask>
          </defs>
          <rect
            x="0"
            y="0"
            width="100%"
            height="100%"
            class="tour-mask__fill"
            :mask="`url(#${maskId})`"
          />
        </svg>
        <!-- 聚光灯描边 -->
        <div
          v-if="spot"
          class="tour-ring"
          :style="{
            left: `${spot.x}px`,
            top: `${spot.y}px`,
            width: `${spot.w}px`,
            height: `${spot.h}px`,
            borderRadius: `${spot.r}px`,
          }"
          aria-hidden="true"
        />

        <!-- 引导卡片 -->
        <section
          ref="cardRef"
          class="tour-card glass-heavy"
          :class="[`is-${placement}`, { 'is-centered': !spot }]"
          :style="cardStyle"
        >
          <header class="tour-card__head">
            <span class="tour-card__step">{{ store.index + 1 }} / {{ store.total }}</span>
            <button class="tour-skip" type="button" @click="store.skip()">
              {{ t('onboarding.skip') }}
            </button>
          </header>

          <div v-if="!spot" class="tour-card__hero" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="40" height="40" fill="none">
              <path
                d="M12 2c1 3.5-1 5.5-3 8-1.8 2.2-2.5 4-2 6.5A5.5 5.5 0 0 0 12 22a5.5 5.5 0 0 0 5-5.5c.5-2.5-.2-4.3-2-6.5-2-2.5-4-4.5-3-8Z"
                fill="currentColor"
                opacity="0.92"
              />
            </svg>
          </div>

          <h3 class="tour-card__title">{{ t(`onboarding.steps.${step.id}.title`) }}</h3>
          <p class="tour-card__body">{{ t(`onboarding.steps.${step.id}.body`) }}</p>

          <div class="tour-dots" aria-hidden="true">
            <button
              v-for="(s, i) in store.steps"
              :key="s.id"
              class="tour-dot"
              :class="{ 'is-active': i === store.index, 'is-done': i < store.index }"
              type="button"
              tabindex="-1"
              @click="store.goTo(i)"
            />
          </div>

          <footer class="tour-card__foot">
            <FpButton v-if="!store.isFirst" variant="ghost" size="small" @click="store.prev()">
              {{ t('onboarding.prev') }}
            </FpButton>
            <span class="tour-spacer" />
            <FpButton ref="nextBtnRef" variant="primary" size="small" @click="store.next()">
              {{
                store.isLast
                  ? t('onboarding.finish')
                  : store.isFirst
                    ? t('onboarding.start')
                    : t('onboarding.next')
              }}
            </FpButton>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'
import { useOnboardingStore } from '@/stores/onboarding'
import { useAuthStore } from '@/stores/auth'

/**
 * 新手操作向导（UX-2）——聚光灯式 coach-marks。
 *
 * 设计要点：
 * - **锚点解耦**：目标元素只需带 `data-tour="<anchor>"`，向导不 import 任何业务组件；
 *   锚点缺失（如该角色看不到某菜单）自动跳过该步，不会卡死；
 * - **测量而不修改 DOM**：用 `getBoundingClientRect` + SVG mask 画洞，目标元素零样式侵入；
 *   `ResizeObserver` + `scroll/resize` 监听下重新测量（节流到一帧）；
 * - **可跳过、可键盘操作**：Esc = 跳过，← → = 上一步/下一步，Enter = 下一步；
 *   焦点进入卡片，读屏可读到步骤序号与标题；
 * - **非阻塞**：遮罩 `pointer-events: none`，只有卡片可交互——用户可以边看提示边点真实控件
 *   （也避免自动化 / 读屏用户被一层全屏遮罩「困住」）；
 * - **首次自动触发**由本组件在挂载时完成（`store.autoStart(username)`），
 *   完成 / 跳过后按用户名持久化，之后只能从用户菜单手动重放。
 */
const { t } = useI18n()
const store = useOnboardingStore()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

const step = computed(() => store.current)
const placement = computed(() => step.value?.placement ?? 'bottom')
const maskId = `tour-mask-${Math.random().toString(36).slice(2, 8)}`

interface Spot {
  x: number
  y: number
  w: number
  h: number
  r: number
}
const spot = ref<Spot | null>(null)
const cardRef = ref<HTMLElement | null>(null)
const nextBtnRef = ref<{ $el?: HTMLElement } | null>(null)
const cardPos = ref<{ left: number; top: number } | null>(null)

const PAD = 8
const GAP = 14
const CARD_W = 340

const cardStyle = computed(() => {
  if (!spot.value || !cardPos.value) return {}
  return { left: `${cardPos.value.left}px`, top: `${cardPos.value.top}px`, width: `${CARD_W}px` }
})

function findAnchor(anchor?: string): HTMLElement | null {
  if (!anchor) return null
  return document.querySelector<HTMLElement>(`[data-tour="${anchor}"]`)
}

/** 测量锚点 → 聚光灯矩形 + 卡片定位（限制在视口内） */
function measure() {
  const s = step.value
  if (!s) return
  const el = findAnchor(s.anchor)
  if (!el) {
    spot.value = null
    cardPos.value = null
    return
  }
  const r = el.getBoundingClientRect()
  const radius = Math.min(12, parseFloat(getComputedStyle(el).borderRadius) || 10)
  spot.value = {
    x: Math.max(0, r.left - PAD),
    y: Math.max(0, r.top - PAD),
    w: r.width + PAD * 2,
    h: r.height + PAD * 2,
    r: radius + 4,
  }
  // 卡片定位：先按偏好位置，越界则翻转 / 夹紧
  const vw = window.innerWidth
  const vh = window.innerHeight
  const ch = cardRef.value?.offsetHeight ?? 220
  let left = 0
  let top = 0
  const sp = spot.value
  switch (placement.value) {
    case 'right':
      left = sp.x + sp.w + GAP
      top = sp.y
      if (left + CARD_W > vw) left = sp.x - CARD_W - GAP
      break
    case 'left':
      left = sp.x - CARD_W - GAP
      top = sp.y
      if (left < 0) left = sp.x + sp.w + GAP
      break
    case 'top':
      left = sp.x + sp.w / 2 - CARD_W / 2
      top = sp.y - ch - GAP
      if (top < 0) top = sp.y + sp.h + GAP
      break
    default:
      left = sp.x + sp.w / 2 - CARD_W / 2
      top = sp.y + sp.h + GAP
      if (top + ch > vh) top = sp.y - ch - GAP
  }
  left = Math.min(Math.max(12, left), Math.max(12, vw - CARD_W - 12))
  top = Math.min(Math.max(12, top), Math.max(12, vh - ch - 12))
  cardPos.value = { left, top }
}

let raf = 0
function scheduleMeasure() {
  if (raf) return
  raf = requestAnimationFrame(() => {
    raf = 0
    measure()
  })
}

/** 步骤切换：先保证路由、再滚动锚点入视、再测量；锚点不存在则自动跳到下一步 */
async function enterStep() {
  const s = step.value
  if (!s) return
  if (s.route && route.path !== s.route) {
    await router.push(s.route).catch(() => {})
    await nextTick()
  }
  // 给懒加载视图一帧渲染时间
  await new Promise((r) => requestAnimationFrame(() => r(null)))
  const el = findAnchor(s.anchor)
  if (s.anchor && !el) {
    // 该角色/布局下锚点不存在（如 viewer 看不到设置）：跳过这一步
    if (store.isLast) store.finish()
    else store.next()
    return
  }
  el?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' })
  measure()
  await nextTick()
  measure()
  focusNext()
}

function focusNext() {
  const btn = nextBtnRef.value?.$el ?? null
  const target = btn instanceof HTMLElement ? btn : cardRef.value
  target?.focus?.({ preventScroll: true })
}

function onKeydown(e: KeyboardEvent) {
  switch (e.key) {
    case 'Escape':
      e.preventDefault()
      store.skip()
      break
    case 'ArrowRight':
    case 'Enter':
      if ((e.target as HTMLElement)?.tagName === 'BUTTON' && e.key === 'Enter') return
      e.preventDefault()
      store.next()
      break
    case 'ArrowLeft':
      e.preventDefault()
      store.prev()
      break
    default:
  }
}

watch(
  () => [store.active, store.index] as const,
  ([active]) => {
    if (active) void enterStep()
    else {
      spot.value = null
      cardPos.value = null
    }
  },
)

let ro: ResizeObserver | null = null
onMounted(() => {
  window.addEventListener('resize', scheduleMeasure)
  window.addEventListener('scroll', scheduleMeasure, true)
  if (typeof ResizeObserver !== 'undefined') {
    ro = new ResizeObserver(scheduleMeasure)
    ro.observe(document.documentElement)
  }
  // 首次进入自动触发（登录态下按用户名判定）
  if (auth.isLoggedIn && !route.path.startsWith('/login')) {
    store.autoStart(auth.username || '')
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', scheduleMeasure)
  window.removeEventListener('scroll', scheduleMeasure, true)
  ro?.disconnect()
  if (raf) cancelAnimationFrame(raf)
})
</script>

<style scoped>
.tour {
  position: fixed;
  inset: 0;
  z-index: var(--fp-z-modal);
  /* 非阻塞式 coach-marks：遮罩与描边不吃事件，页面仍可点击（用户可边看边试）；只有卡片可交互 */
  pointer-events: none;
}
.tour-card {
  pointer-events: auto;
}
.tour-mask {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}
.tour-mask__fill {
  fill: oklch(0.1 0.01 250 / 0.62);
}
.tour-ring {
  position: absolute;
  box-shadow:
    0 0 0 2px var(--fp-brand),
    0 0 0 6px var(--fp-brand-soft),
    var(--fp-shadow-brand);
  pointer-events: none;
  transition:
    left var(--fp-transition-base),
    top var(--fp-transition-base),
    width var(--fp-transition-base),
    height var(--fp-transition-base);
}

.tour-card {
  position: absolute;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4) var(--fp-space-5) var(--fp-space-4);
  border-radius: var(--fp-radius-lg);
  color: var(--fp-text-primary);
  outline: none;
  transition:
    left var(--fp-transition-base),
    top var(--fp-transition-base);
}
.tour-card.is-centered {
  left: 50%;
  top: 50%;
  width: min(420px, calc(100vw - 32px));
  transform: translate(-50%, -50%);
  text-align: center;
  align-items: center;
  padding: var(--fp-space-6) var(--fp-space-6) var(--fp-space-5);
}
.tour-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
.tour-card__step {
  font-family: var(--fp-font-mono);
  font-size: 11px;
  letter-spacing: 0.06em;
  color: var(--fp-text-muted);
}
.tour-skip {
  border: none;
  background: transparent;
  color: var(--fp-text-secondary);
  font: inherit;
  font-size: 12.5px;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: var(--fp-radius-sm);
  transition:
    color var(--fp-transition-fast),
    background-color var(--fp-transition-fast);
}
.tour-skip:hover {
  color: var(--fp-text-primary);
  background: var(--fp-bg-hover);
}
.tour-card__hero {
  color: var(--fp-brand);
  filter: drop-shadow(0 4px 16px var(--fp-brand-soft));
}
.tour-card__title {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.tour-card__body {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--fp-text-secondary);
}
.tour-dots {
  display: flex;
  gap: 6px;
  justify-content: center;
  width: 100%;
}
.tour-dot {
  width: 6px;
  height: 6px;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: var(--fp-border-strong);
  cursor: pointer;
  transition:
    width var(--fp-transition-fast),
    background-color var(--fp-transition-fast);
}
.tour-dot.is-done {
  background: color-mix(in oklch, var(--fp-brand) 55%, var(--fp-border-strong));
}
.tour-dot.is-active {
  width: 18px;
  background: var(--fp-brand);
}
.tour-card__foot {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  width: 100%;
}
.tour-spacer {
  flex: 1;
}

.tour-enter-active,
.tour-leave-active {
  transition: opacity var(--fp-transition-base);
}
.tour-enter-active .tour-card,
.tour-leave-active .tour-card {
  transition:
    opacity var(--fp-transition-base),
    transform var(--fp-transition-base);
}
.tour-enter-from,
.tour-leave-to {
  opacity: 0;
}
.tour-enter-from .tour-card:not(.is-centered) {
  transform: translateY(6px);
}

@media (prefers-reduced-motion: reduce) {
  .tour-ring,
  .tour-card,
  .tour-dot {
    transition: none;
  }
}
</style>
