<template>
  <div class="fp-switch" :class="{ 'fp-switch--labeled': !!label }">
    <label v-if="label" class="fp-switch__label" :for="uid">{{ label }}</label>
    <ToggleSwitch
      :input-id="inputId || uid"
      :model-value="model"
      :disabled="disabled"
      @update:model-value="emit"
    />
    <small v-if="hint" class="fp-switch__hint">{{ hint }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import ToggleSwitch from 'openvue/toggleswitch'

withDefaults(
  defineProps<{
    disabled?: boolean
    /** 左侧标签；给定后开关渲染为「标签 + 开关」一行，点击标签可切换 */
    label?: string
    /** 开关右侧的辅助说明 */
    hint?: string
    /** 外部 `<label for>` 关联用 */
    inputId?: string
  }>(),
  { disabled: false, label: '', hint: '', inputId: '' },
)

const model = defineModel<boolean>({ default: false })
const uid = useId()

function emit(v: boolean | undefined) {
  model.value = !!v
}
</script>

<style scoped>
.fp-switch {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-3);
}
.fp-switch--labeled {
  display: flex;
}
.fp-switch__label {
  font-size: 13px;
  font-weight: 500;
  color: var(--fp-text-secondary);
  cursor: pointer;
}
.fp-switch__hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}
</style>
