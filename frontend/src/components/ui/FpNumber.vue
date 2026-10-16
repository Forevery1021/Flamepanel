<template>
  <div class="fp-number" :class="{ 'fp-field-invalid': !!error || invalid }">
    <label v-if="label" class="fp-field__label" :for="uid">{{ label }}</label>
    <InputNumber
      :input-id="inputId || uid"
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      class="w-full"
      @update:model-value="emit"
    />
    <small v-if="error" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import InputNumber from 'openvue/inputnumber'

withDefaults(
  defineProps<{
    /** 外部 `<label for>` 关联用：指定内部可聚焦控件的 id（默认自动生成） */
    inputId?: string
    label?: string
    error?: string
    invalid?: boolean
  }>(),
  { label: '', error: '', invalid: false, inputId: '' },
)

const model = defineModel<number | null>()
const uid = useId()

function emit(value: number | null | undefined) {
  model.value = value ?? null
}
</script>

<style scoped>
.fp-number {
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
.fp-field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
