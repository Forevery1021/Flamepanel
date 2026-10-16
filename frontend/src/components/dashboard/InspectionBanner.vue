<template>
  <div
    v-if="visible && report"
    class="inspection-banner"
    role="status"
    data-testid="inspection-banner"
  >
    <i class="oi oi-exclamation-triangle banner-icon" aria-hidden="true" />
    <div class="banner-body">
      <div class="banner-title">{{ t('inspection.banner.title', { count: urgent.length }) }}</div>
      <div class="banner-items">
        <span v-for="f in preview" :key="`${f.code}:${f.subject ?? ''}`" class="banner-item">
          <FpTag
            :severity="severityTone(f.severity)"
            :value="t(`inspection.severity.${f.severity}`)"
          />
          <span>{{ t(findingI18nKey(f.code, 'title'), { code: f.code }) }}</span>
          <code v-if="f.subject" class="mono">{{ f.subject }}</code>
        </span>
        <span v-if="urgent.length > preview.length" class="banner-more">
          +{{ urgent.length - preview.length }}
        </span>
      </div>
    </div>
    <div class="banner-actions">
      <FpButton variant="primary" size="small" @click="router.push('/inspection')">
        {{ t('inspection.banner.view') }}
      </FpButton>
      <FpButton variant="ghost" size="small" @click="dismiss">
        {{ t('inspection.banner.dismiss') }}
      </FpButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useInspection } from '@/composables/useInspection'
import { findingI18nKey, severityTone } from '@/utils/inspection'
import { STORAGE_KEYS } from '@/utils/storage'

/**
 * S-17 仪表盘安全体检横幅：只在最近一次体检存在 critical / high 风险时出现。
 *
 * - 与体检页共享同一份查询缓存（同 queryKey），仪表盘不会额外触发一次采集；
 * - 「本次忽略」按 `generated_at` 记在 sessionStorage：同一份报告本会话不再打扰，
 *   下一次体检（新的 generated_at）若仍有高危项会再次出现。
 */
const PREVIEW_COUNT = 3

const { t } = useI18n()
const router = useRouter()
const { report, urgent } = useInspection()

function readDismissed(): string {
  try {
    return sessionStorage.getItem(STORAGE_KEYS.inspectionBannerDismissed) ?? ''
  } catch {
    return ''
  }
}
const dismissedAt = ref(readDismissed())

const visible = computed(
  () => urgent.value.length > 0 && report.value?.generated_at !== dismissedAt.value,
)
const preview = computed(() => urgent.value.slice(0, PREVIEW_COUNT))

function dismiss() {
  const stamp = report.value?.generated_at ?? ''
  dismissedAt.value = stamp
  try {
    sessionStorage.setItem(STORAGE_KEYS.inspectionBannerDismissed, stamp)
  } catch {
    /* 隐私模式等不可写：仅本次渲染内生效 */
  }
}
</script>

<style scoped>
.inspection-banner {
  display: flex;
  align-items: flex-start;
  gap: var(--fp-space-3);
  padding: var(--fp-space-3) var(--fp-space-4);
  border: 1px solid color-mix(in oklch, var(--fp-danger) 40%, var(--fp-border));
  border-radius: var(--fp-radius-md);
  background: var(--fp-danger-soft);
}
.banner-icon {
  flex: 0 0 auto;
  margin-top: 2px;
  font-size: 18px;
  color: var(--fp-danger);
}
.banner-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-1);
}
.banner-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.banner-items {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2) var(--fp-space-4);
  font-size: 12.5px;
  color: var(--fp-text-secondary);
}
.banner-item {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-1);
}
.banner-item code {
  font-size: 12px;
  color: var(--fp-text-code);
}
.banner-more {
  color: var(--fp-text-muted);
}
.banner-actions {
  flex: 0 0 auto;
  display: flex;
  gap: var(--fp-space-2);
}
@media (max-width: 720px) {
  .inspection-banner {
    flex-wrap: wrap;
  }
  .banner-actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
