<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <label v-if="label" class="fp-field__label" :for="uid">{{ label }}</label>
    <Textarea
      :id="inputId || uid"
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      :aria-invalid="!!error || invalid || undefined"
      :aria-describedby="error ? `${uid}-error` : undefined"
      class="w-full"
      @update:model-value="emit"
    />
    <small v-if="error" :id="`${uid}-error`" class="fp-textarea-error">{{ error }}</small>
    <small v-else-if="hint" class="fp-field__hint">{{ hint }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import Textarea from 'openvue/textarea'

withDefaults(
  defineProps<{
    /** 外部 `<label for>` 关联用：指定内部可聚焦控件的 id（默认自动生成） */
    inputId?: string
    label?: string
    hint?: string
    error?: string
    invalid?: boolean
  }>(),
  { label: '', hint: '', error: '', invalid: false, inputId: '' },
)

const model = defineModel<string>()
const uid = useId()
const emit = (v: string | undefined) => {
  model.value = v
}
</script>

<style scoped>
.fp-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}
.fp-field__label {
  font-size: 13px;
  font-weight: 500;
  color: var(--fp-text-secondary);
}
.fp-field__hint {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-text-muted);
}
.fp-textarea-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
  margin-top: 4px;
}
</style>
