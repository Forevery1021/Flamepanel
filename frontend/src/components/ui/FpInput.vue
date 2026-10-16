<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <label v-if="label" class="fp-field__label" :for="uid">{{ label }}</label>
    <InputText
      :id="inputId || uid"
      v-bind="$attrs"
      :aria-label="accessibleName"
      :model-value="model"
      :invalid="!!error || invalid"
      :aria-invalid="!!error || invalid || undefined"
      :aria-describedby="error ? `${uid}-error` : undefined"
      class="w-full"
      :class="{ 'text-right': align === 'right' }"
      @update:model-value="emit"
    />
    <small v-if="error" :id="`${uid}-error`" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { computed, useAttrs, useId } from 'vue'
import InputText from 'openvue/inputtext'

const props = withDefaults(
  defineProps<{
    /** 外部 `<label for>` 关联用：指定内部可聚焦控件的 id（默认自动生成） */
    inputId?: string
    label?: string
    error?: string
    invalid?: boolean
    align?: 'left' | 'right'
  }>(),
  { label: '', error: '', invalid: false, align: 'left', inputId: '' },
)

const model = defineModel<string>()
const attrs = useAttrs()

/**
 * a11y 审计（batch-10）：工具栏搜索框等常只给 placeholder 不给可见 label。
 * 无 label 时把显式 aria-label、否则 placeholder 作为可访问名称，读屏不再播报「编辑框，空白」。
 * 有可见 label 时由 `<label for>` 命名，不重复设置。
 */
const accessibleName = computed(() => {
  if (props.label) return undefined
  const explicit = attrs['aria-label']
  if (typeof explicit === 'string' && explicit) return explicit
  const placeholder = attrs.placeholder
  return typeof placeholder === 'string' && placeholder ? placeholder : undefined
})
const uid = useId()

function emit(value: string | undefined) {
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
