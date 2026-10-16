<template>
  <div class="layout-content">
    <header class="lc-header">
      <div class="lc-heading">
        <h2 class="lc-title__text">{{ title }}</h2>
        <p v-if="description" class="lc-desc">{{ description }}</p>
        <div v-if="$slots.prompt" class="lc-prompt">
          <slot name="prompt" />
        </div>
      </div>
      <div class="lc-actions">
        <slot name="actions" />
        <FpButton
          v-if="reload"
          variant="ghost"
          icon="oi oi-refresh"
          :loading="reloading"
          :aria-label="t('common.refresh')"
          :title="t('common.refresh')"
          @click="doReload"
        />
      </div>
    </header>

    <!-- 页级摘要：KPI pills（总数 / 运行中 / 异常 …） -->
    <div v-if="summary?.length || $slots.summary" class="lc-summary">
      <slot name="summary">
        <span
          v-for="s in summary"
          :key="s.label"
          class="lc-pill"
          :class="s.tone ? `is-${s.tone}` : ''"
        >
          <span class="lc-pill__dot" aria-hidden="true" />
          <span class="lc-pill__value">{{ s.value }}</span>
          <span class="lc-pill__label">{{ s.label }}</span>
        </span>
      </slot>
    </div>

    <div v-if="$slots.toolbar" class="lc-toolbar">
      <slot name="toolbar" />
    </div>

    <div class="lc-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpButton from './FpButton.vue'

export interface LayoutSummaryItem {
  label: string
  value: string | number
  tone?: 'brand' | 'success' | 'warning' | 'danger' | 'info' | 'muted'
}

const props = withDefaults(
  defineProps<{
    title?: string
    /** 标题下一行说明（页面用途 / 当前范围） */
    description?: string
    /** 页级 KPI 摘要 */
    summary?: LayoutSummaryItem[]
    /** 是否显示刷新按钮 */
    reload?: boolean
  }>(),
  { title: '', description: '', summary: () => [], reload: false },
)

const emit = defineEmits<{ reload: [] }>()

const { t } = useI18n()
const reloading = ref(false)

async function doReload() {
  if (props.reload) {
    reloading.value = true
    emit('reload')
    await new Promise((r) => setTimeout(r, 500))
    reloading.value = false
  }
}
</script>

<style scoped>
.layout-content {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  min-width: 0;
}
.lc-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--fp-space-4);
}
.lc-heading {
  min-width: 0;
  flex: 1 1 auto;
}
.lc-title__text {
  margin: 0;
  font-size: var(--fp-font-title);
  font-weight: 700;
  letter-spacing: -0.015em;
  line-height: 1.2;
  color: var(--fp-text-primary);
}
.lc-desc {
  margin: var(--fp-space-1) 0 0;
  font-size: var(--fp-font-sm);
  color: var(--fp-text-secondary);
}
.lc-prompt {
  margin-top: var(--fp-space-2);
  max-width: 720px;
}
.lc-actions {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex: 0 0 auto;
  white-space: nowrap;
}
.lc-summary {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
}
.lc-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 26px;
  padding: 0 10px 0 8px;
  border: 1px solid var(--fp-border);
  border-radius: 999px;
  background: var(--fp-bg-surface);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-secondary);
  --pill: var(--fp-text-muted);
}
.lc-pill.is-brand {
  --pill: var(--fp-brand);
}
.lc-pill.is-success {
  --pill: var(--fp-success);
}
.lc-pill.is-warning {
  --pill: var(--fp-warning);
}
.lc-pill.is-danger {
  --pill: var(--fp-danger);
}
.lc-pill.is-info {
  --pill: var(--fp-info);
}
.lc-pill__dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--pill);
}
.lc-pill__value {
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--fp-text-primary);
}
.lc-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fp-space-3);
  flex-wrap: wrap;
}
.lc-toolbar > :deep(.toolbar-left),
.lc-toolbar > :deep(.toolbar-right) {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.lc-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  min-width: 0;
}
@media (max-width: 768px) {
  .lc-header {
    flex-direction: column;
  }
  .lc-actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
