<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.welcomeTitle') }}</h2>
    <p class="step__sub">{{ t('setup.welcomeSub') }}</p>

    <div class="step-env">
      <div class="step-env__item">
        <span class="step-env__icon"><i class="oi oi-database" /></span>
        <div>
          <p class="step-env__name">{{ t('setup.docker') }}</p>
          <FpChip
            :label="docker === null ? t('setup.unknown') : docker ? t('setup.available') : t('setup.unavailable')"
            :class="docker === null ? '' : docker ? 'chip-ok' : 'chip-warn'"
          />
        </div>
      </div>
      <div class="step-env__item">
        <span class="step-env__icon"><i class="oi oi-server" /></span>
        <div>
          <p class="step-env__name">{{ t('setup.nginx') }}</p>
          <FpChip
            :label="nginx === null ? t('setup.unknown') : nginx ? t('setup.available') : t('setup.unavailable')"
            :class="nginx === null ? '' : nginx ? 'chip-ok' : 'chip-warn'"
          />
        </div>
      </div>
    </div>

    <p class="step-hint">{{ t('setup.welcomeHint') }}</p>

    <FpButton variant="primary" size="large" class="step-cta" @click="$emit('continue')">
      {{ t('setup.start') }}
    </FpButton>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'
import FpChip from '@/components/ui/FpChip.vue'

defineProps<{
  docker: boolean | null
  nginx: boolean | null
}>()

defineEmits<{ continue: [] }>()

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
.step-env {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--fp-space-3);
  margin-top: var(--fp-space-2);
}
.step-env__item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.step-env__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  font-size: 16px;
}
.step-env__name {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: 4px;
}
.chip-ok :deep(.fp-chip) {
  background: var(--fp-success-soft);
  color: var(--fp-success);
}
.chip-warn :deep(.fp-chip) {
  background: var(--fp-warning-soft);
  color: var(--fp-warning);
}
.step-hint {
  font-size: 12.5px;
  color: var(--fp-text-secondary);
  line-height: 1.6;
}
.step-cta {
  align-self: flex-start;
  margin-top: var(--fp-space-2);
}

@media (max-width: 480px) {
  .step-env {
    grid-template-columns: 1fr;
  }
}
</style>
