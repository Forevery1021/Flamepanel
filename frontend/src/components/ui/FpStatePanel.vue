<template>
  <div class="fp-state">
    <!-- 加载态：骨架优先 -->
    <div v-if="loading" class="fp-state__loading" role="status" aria-live="polite">
      <slot name="loading">
        <div class="fp-state__spinner">
          <i class="oi oi-spinner fp-spin" />
        </div>
        <p v-if="loadingText" class="fp-state__text">{{ loadingText }}</p>
      </slot>
    </div>

    <!-- 错误态：可重试 -->
    <div v-else-if="error" class="fp-state__error" role="alert">
      <slot name="error">
        <div class="fp-state__icon">
          <i class="oi oi-cloud-off" />
        </div>
        <p class="fp-state__title">{{ title || t('common.failed') }}</p>
        <p v-if="description || error" class="fp-state__desc">{{ description || error }}</p>
        <FpButton
          v-if="retryable"
          variant="secondary"
          icon="oi oi-refresh"
          :loading="retrying"
          @click="doRetry"
        >
          {{ retryText || t('common.retry') }}
        </FpButton>
      </slot>
    </div>

    <!-- 空态 -->
    <div v-else-if="empty && !hideEmpty" class="fp-state__empty">
      <slot name="empty">
        <FpEmpty :icon="emptyIcon" :title="emptyTitle || t('common.noData')" :description="emptyDesc">
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
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpButton from './FpButton.vue'
import FpEmpty from './FpEmpty.vue'

withDefaults(
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
  },
)

const emit = defineEmits<{ retry: [] }>()
const { t } = useI18n()
const retrying = ref(false)

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
