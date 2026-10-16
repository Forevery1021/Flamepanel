<template>
  <div v-if="fatal" class="fp-eb" role="alert">
    <div class="fp-eb__box">
      <div class="fp-eb__icon" aria-hidden="true">
        <i class="oi oi-exclamation-triangle" />
      </div>
      <p class="fp-eb__title">{{ tt('common.renderError.title', fb.renderTitle) }}</p>
      <p class="fp-eb__desc">{{ message }}</p>
      <div class="fp-eb__actions">
        <button type="button" class="fp-eb__btn" @click="retry">
          {{ tt('common.retry', fb.retry) }}
        </button>
        <button type="button" class="fp-eb__btn fp-eb__btn--primary" @click="reload">
          {{ tt('common.renderError.reload', fb.reload) }}
        </button>
      </div>
    </div>
  </div>
  <slot v-else />
</template>

<script setup lang="ts">
import { computed, onErrorCaptured, ref, watch } from 'vue'
import { useSafeText } from '@/composables/useSafeText'
import { fallbackText } from '@/utils/fallbackText'
import { reportError } from '@/utils/monitor'

/**
 * R-2：可复用的渲染错误边界（Doc/23 §2.5）。
 *
 * 设计要点：
 * 1. **边界下沉**：包裹 router-view（而非整棵布局树），页面崩溃时导航骨架、
 *    顶栏、页签仍在——用户可直接切到别的菜单自救，而不是整屏白/黑。
 * 2. **文案自证**：面板文案走 `useSafeText`（i18n 未就绪时回落硬编码兜底文案），
 *    避免语言包未加载时上屏裸 key（`common.failed`）或整块空白。
 * 3. **零第三方依赖**：按钮用原生 `<button>` + 令牌自绘，不经 OpenVue（FpButton）。
 *    兜底层必须比它保护的对象更稳——若崩溃源恰是 beta 依赖（Doc/23 P2-7），
 *    依赖它的兜底面板会同归于尽。本组件仅依赖 Vue 与项目内工具。
 * 4. **错误可恢复**：捕获后 `return false` 阻止继续上抛；「重试」仅重挂子树
 *    （不刷新页面，保留其它页签与未保存状态）；「刷新页面」供不可恢复场景兜底。
 * 5. **无 router 依赖**：复位靠 `resetKey` 外部驱动（如传 `route.fullPath`），
 *    组件本身不读路由，便于在任意上下文（含单测）复用。
 *
 * 说明：`onErrorCaptured` 沿组件父链冒泡，捕获范围仅限本边界子树；
 * `return false` 同时会阻止 `app.config.errorHandler` 被调用，故不会重复上报。
 */
const props = withDefaults(
  defineProps<{
    /** 是否上报错误（默认 true，走 utils/monitor 全局单点） */
    report?: boolean
    /** 复位键：值变化时清除错误态（通常传当前路由，切页后不残留错误面板） */
    resetKey?: string | number
  }>(),
  { report: true, resetKey: undefined },
)

const tt = useSafeText()
const fb = fallbackText()

const fatal = ref<unknown>(null)

const message = computed(() => {
  const err = fatal.value
  if (err instanceof Error) return err.message
  if (typeof err === 'string') return err
  return String(err)
})

onErrorCaptured((err, _instance, info) => {
  if (props.report) {
    reportError(err, { source: 'render', context: info })
  }
  fatal.value = err
  // 阻止继续向上冒泡：由本边界接管展示与恢复
  return false
})

function retry() {
  fatal.value = null
}

function reload() {
  if (typeof window !== 'undefined') window.location.reload()
}

// 复位键变化即清错：避免用户切走后回到本页仍停在旧错误面板
watch(
  () => props.resetKey,
  () => {
    fatal.value = null
  },
)
</script>

<style scoped>
.fp-eb {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 280px;
  padding: var(--fp-space-8) var(--fp-space-4);
}
.fp-eb__box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--fp-space-3);
  max-width: 480px;
  padding: var(--fp-space-8);
  text-align: center;
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-lg);
  box-shadow: var(--fp-shadow-sm);
}
.fp-eb__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 999px;
  background: var(--fp-danger-soft);
  color: var(--fp-danger);
  font-size: 22px;
}
.fp-eb__title {
  font-size: var(--fp-font-md);
  font-weight: 600;
  color: var(--fp-text-primary);
}
.fp-eb__desc {
  font-size: var(--fp-font-sm);
  color: var(--fp-text-secondary);
  word-break: break-word;
}
.fp-eb__actions {
  display: flex;
  gap: var(--fp-space-2);
  margin-top: var(--fp-space-2);
}
/* 自绘按钮：不依赖底层组件库，见组件注释「零第三方依赖」 */
.fp-eb__btn {
  padding: var(--fp-space-2) var(--fp-space-4);
  font-family: inherit;
  font-size: var(--fp-font-sm);
  color: var(--fp-text-primary);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border-strong);
  border-radius: var(--fp-radius-sm);
  cursor: pointer;
  transition:
    background var(--fp-transition-fast),
    border-color var(--fp-transition-fast);
}
.fp-eb__btn:hover {
  background: var(--fp-bg-hover);
}
.fp-eb__btn--primary {
  color: var(--fp-text-invert);
  background: var(--fp-brand);
  border-color: var(--fp-brand);
}
.fp-eb__btn--primary:hover {
  background: var(--fp-brand-strong);
  border-color: var(--fp-brand-strong);
}
</style>
