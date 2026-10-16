<template>
  <div class="fp-state">
    <!-- 加载态：骨架优先 -->
    <div v-if="loading" class="fp-state__loading" role="status" aria-live="polite">
      <slot name="loading">
        <div v-if="skeleton" class="fp-state__skeleton" aria-hidden="true">
          <div v-for="i in skeletonRows" :key="i" class="fp-state__skeleton-row">
            <FpSkeleton width="18%" height="12px" />
            <FpSkeleton width="30%" height="12px" />
            <FpSkeleton width="14%" height="12px" />
            <FpSkeleton width="22%" height="12px" />
          </div>
        </div>
        <template v-else>
          <div class="fp-state__spinner">
            <i class="oi oi-spinner fp-spin" />
          </div>
          <p v-if="loadingText" class="fp-state__text">{{ loadingText }}</p>
        </template>
      </slot>
    </div>

    <!-- 错误态：可重试 -->
    <div v-else-if="showError" class="fp-state__error" role="alert">
      <slot name="error">
        <div class="fp-state__icon">
          <i class="oi oi-cloud-off" />
        </div>
        <p class="fp-state__title">{{ title || tt('common.failed', fb.failed) }}</p>
        <p v-if="description || error" class="fp-state__desc">{{ description || error }}</p>
        <FpButton
          v-if="retryable"
          variant="secondary"
          icon="oi oi-refresh"
          :loading="retrying"
          @click="doRetry"
        >
          {{ retryText || tt('common.retry', fb.retry) }}
        </FpButton>
      </slot>
    </div>

    <!-- 空态 -->
    <div v-else-if="empty && !hideEmpty" class="fp-state__empty">
      <slot name="empty">
        <FpEmpty
          :icon="emptyIcon"
          :title="emptyTitle || tt('common.noData', fb.noData)"
          :description="emptyDesc"
        >
          <template v-if="$slots.emptyAction" #action>
            <slot name="emptyAction" />
          </template>
        </FpEmpty>
      </slot>
    </div>

    <!-- 默认内容 -->
    <slot v-else />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import FpButton from './FpButton.vue'
import FpEmpty from './FpEmpty.vue'
import FpSkeleton from './FpSkeleton.vue'
import { useSafeText } from '@/composables/useSafeText'
import { fallbackText } from '@/utils/fallbackText'

const props = withDefaults(
  defineProps<{
    /** 加载中 */
    loading?: boolean
    /** 错误信息（非空即错误态） */
    error?: string | null
    /** 空态判定：数据为空（配合 loading/error 后判断） */
    empty?: boolean
    /** 空态时是否隐藏（默认 false） */
    hideEmpty?: boolean
    /** 错误态标题 */
    title?: string
    /** 错误态描述（默认取 error） */
    description?: string
    /** 是否显示重试按钮 */
    retryable?: boolean
    retryText?: string
    loadingText?: string
    /** 空态图标（openicons） */
    emptyIcon?: string
    emptyTitle?: string
    emptyDesc?: string
    /** 加载态用表格骨架（列表页默认开启） */
    skeleton?: boolean
    skeletonRows?: number
  }>(),
  {
    loading: false,
    error: null,
    empty: false,
    hideEmpty: false,
    title: '',
    description: '',
    retryable: false,
    retryText: '',
    loadingText: '',
    emptyIcon: 'oi oi-inbox',
    emptyTitle: '',
    emptyDesc: '',
    skeleton: true,
    skeletonRows: 5,
  },
)

const emit = defineEmits<{ retry: [] }>()
/**
 * R-2：文案经 useSafeText 取词——语言包未加载 / i18n 异常时回落硬编码文案，
 * 避免错误态在致命场景上屏裸 key（`common.failed`）。
 */
const tt = useSafeText()
const fb = fallbackText()
const retrying = ref(false)

/**
 * 错误态判定（R-2 修复）：`error` 非空，或显式传入了错误态标题（`title`）。
 * 历史实现只认 `error`，导致「传 title/description 不传 error」时三态全不命中、
 * 渲染空内容——App 级致命错误兜底曾因此显示为空白。
 */
const showError = computed(() => !!props.error || !!props.title)

async function doRetry() {
  retrying.value = true
  emit('retry')
  // 由父组件刷新后 loading 结束；这里延时复位防止重复点击
  setTimeout(() => (retrying.value = false), 600)
}
</script>

<style scoped>
.fp-state {
  width: 100%;
}
.fp-state__skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  width: 100%;
  padding: var(--fp-space-4);
}
.fp-state__skeleton-row {
  display: flex;
  gap: var(--fp-space-4);
  align-items: center;
}
.fp-state__loading,
.fp-state__error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-8) var(--fp-space-4);
  text-align: center;
}
.fp-state__spinner {
  color: var(--fp-brand);
  font-size: 28px;
}
.fp-state__icon {
  color: var(--fp-danger);
  font-size: 28px;
}
.fp-state__title {
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.fp-state__desc {
  font-size: 13px;
  color: var(--fp-text-secondary);
  max-width: 480px;
  word-break: break-word;
}
.fp-state__text {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.fp-state__empty {
  padding: var(--fp-space-4) 0;
}
.fp-spin {
  animation: fp-rotate 0.8s linear infinite;
}
@keyframes fp-rotate {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .fp-spin {
    animation: none;
  }
}
</style>
