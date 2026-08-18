<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.adminTitle') }}</h2>
    <p class="step__sub">{{ t('setup.adminSub') }}</p>

    <div class="step-form">
      <FpInput
        v-model="username"
        :label="t('setup.adminUsername')"
        :error="errors.username"
        autocomplete="username"
        @update:model-value="emit('update', { username: $event })"
      />
      <FpInput
        v-model="password"
        :label="t('setup.adminPassword')"
        type="password"
        toggle-mask
        :error="errors.password"
        autocomplete="new-password"
        @update:model-value="emit('update', { password: $event })"
      />
      <FpInput
        v-model="confirm"
        :label="t('setup.adminConfirm')"
        type="password"
        toggle-mask
        :error="errors.confirm"
        autocomplete="new-password"
        @update:model-value="emit('update', { confirm: $event })"
      />
    </div>

    <p class="step-hint">{{ t('setup.adminHint') }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpInput from '@/components/ui/FpInput.vue'

const props = defineProps<{
  username: string
  password: string
  confirm: string
  errors: { username: string; password: string; confirm: string }
}>()

const emit = defineEmits<{
  update: [patch: Partial<{ username: string; password: string; confirm: string }>]
}>()

const { t } = useI18n()
// 本地 v-model 与父级同步（输入即时校验）
const username = computed({
  get: () => props.username,
  set: (v) => emit('update', { username: v }),
})
const password = computed({
  get: () => props.password,
  set: (v) => emit('update', { password: v }),
})
const confirm = computed({
  get: () => props.confirm,
  set: (v) => emit('update', { confirm: v }),
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
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  max-width: 320px;
  margin-top: var(--fp-space-2);
}
.step-hint {
  font-size: 12.5px;
  color: var(--fp-text-secondary);
}
</style>
