<template>
  <div class="fp-form-field" :class="{ 'fp-form-field-invalid': !!error }">
    <label v-if="label" class="fp-form-field__label" :for="uid">{{ label }}</label>
    <slot />
    <small v-if="hint && !error" class="fp-form-field__hint">{{ hint }}</small>
    <small v-if="error" :id="`${uid}-error`" class="fp-form-field__error">{{
      error
    }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'

withDefaults(
  defineProps<{
    label?: string
    error?: string
    hint?: string
  }>(),
  { label: '', error: '', hint: '' },
)

const uid = useId()
</script>

<style scoped>
.fp-form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}
.fp-form-field__label {
  font-size: 13px;
  font-weight: 500;
  color: var(--fp-text-secondary);
}
.fp-form-field__hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.fp-form-field__error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
