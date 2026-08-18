<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.serverTitle') }}</h2>
    <p class="step__sub">{{ t('setup.serverSub') }}</p>

    <div class="step-form">
      <FpInput
        v-model="port"
        :label="t('setup.panelPort')"
        type="number"
        @update:model-value="emit('update', { panel_port: Number($event) || 8080 })"
      />
    </div>

    <div class="step-env">
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

    <p class="step-hint">{{ t('setup.serverHint') }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpInput from '@/components/ui/FpInput.vue'
import FpChip from '@/components/ui/FpChip.vue'

const props = defineProps<{
  port: number
  nginx: boolean | null
}>()

const emit = defineEmits<{ update: [patch: Partial<{ panel_port: number }>] }>()

const { t } = useI18n()
const port = computed({
  get: () => String(props.port),
  set: (v) => emit('update', { panel_port: Number(v) || 8080 }),
})
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
.step-form {
  max-width: 320px;
  margin-top: var(--fp-space-2);
}
.step-env {
  display: grid;
  gap: var(--fp-space-3);
  max-width: 320px;
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
</style>
