<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <label v-if="label" class="fp-field__label" :for="uid">{{ label }}</label>
    <Select
      :label-id="inputId || uid"
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      :aria-invalid="!!error || invalid || undefined"
      :aria-describedby="error ? `${uid}-error` : undefined"
      :show-clear="showClear"
      :filter="filter"
      class="w-full"
      @update:model-value="emit"
    />
    <small v-if="error" :id="`${uid}-error`" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import Select from 'openvue/select'

withDefaults(
  defineProps<{
    /** 外部 `<label for>` 关联用：指定内部可聚焦控件的 id（默认自动生成） */
    inputId?: string
    label?: string
    error?: string
    invalid?: boolean
    showClear?: boolean
    filter?: boolean
    multiple?: boolean
  }>(),
  {
    label: '',
    error: '',
    invalid: false,
    showClear: false,
    filter: false,
    multiple: false,
    inputId: '',
  },
)

const model = defineModel<string | number | string[]>()
const uid = useId()

function emit(value: string | number | string[] | undefined) {
  model.value = value
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
.fp-field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
