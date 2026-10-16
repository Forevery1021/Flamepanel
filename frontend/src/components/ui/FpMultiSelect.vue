<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <label v-if="label" class="fp-field__label" :for="inputId || uid">{{ label }}</label>
    <MultiSelect
      :input-id="inputId || uid"
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      :aria-invalid="!!error || invalid || undefined"
      :aria-describedby="error ? `${uid}-error` : undefined"
      :filter="filter"
      class="w-full"
      @update:model-value="emit"
    />
    <small v-if="error" :id="`${uid}-error`" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import MultiSelect from 'openvue/multiselect'

/**
 * FpMultiSelect — 多选下拉（W4-a 新增）。
 *
 * 旧版「隐藏菜单分组」用 `FpSelect multiple`，而底层 Select 是单选组件，`multiple` 被静默忽略，
 * 只能选中一个分组。多选语义统一走本组件。
 */
withDefaults(
  defineProps<{
    label?: string
    error?: string
    invalid?: boolean
    filter?: boolean
    /** 外部 `<label for>` 关联用：指定内部可聚焦控件的 id（默认自动生成） */
    inputId?: string
  }>(),
  { label: '', error: '', invalid: false, filter: false, inputId: '' },
)

const model = defineModel<Array<string | number>>({ default: () => [] })
const uid = useId()

function emit(value: Array<string | number> | undefined | null) {
  model.value = value ?? []
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
