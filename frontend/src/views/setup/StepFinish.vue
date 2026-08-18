<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.finishTitle') }}</h2>
    <p class="step__sub">{{ t('setup.finishSub') }}</p>

    <dl class="step-summary">
      <div class="summary-row">
        <dt>{{ t('setup.summaryAdmin') }}</dt>
        <dd>{{ summary.username }}</dd>
      </div>
      <div class="summary-row">
        <dt>{{ t('setup.summaryDatabase') }}</dt>
        <dd>{{ summary.dbType }}{{ summary.dbName !== 'SQLite' ? ` / ${summary.dbName}` : '' }}</dd>
      </div>
      <div class="summary-row">
        <dt>{{ t('setup.summaryTheme') }}</dt>
        <dd>{{ t(`settingsTheme.${summary.theme}`) }}</dd>
      </div>
      <div class="summary-row">
        <dt>{{ t('setup.summaryLanguage') }}</dt>
        <dd>{{ summary.language }}</dd>
      </div>
    </dl>

    <p v-if="error" class="step-error">
      <i class="oi oi-exclamation-triangle" />
      {{ error }}
    </p>

    <FpButton variant="primary" size="large" :loading="submitting" @click="$emit('submit')">
      {{ submitting ? t('setup.installing') : t('setup.install') }}
    </FpButton>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'

defineProps<{
  summary: {
    username: string
    dbType: string
    dbName: string
    theme: string
    language: string
  }
  submitting: boolean
  error: string
}>()

defineEmits<{ submit: [] }>()

const { t } = useI18n()
</script>

<style scoped>
.step {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.step__title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fp-text-primary);
}
.step__sub {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.step-summary {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  max-width: 360px;
}
.summary-row {
  display: flex;
  justify-content: space-between;
  gap: var(--fp-space-3);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.summary-row dt {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.summary-row dd {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.step-error {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  background: var(--fp-danger-soft);
  color: var(--fp-danger);
  font-size: 13px;
}
.step-error i {
  font-size: 14px;
}
</style>
