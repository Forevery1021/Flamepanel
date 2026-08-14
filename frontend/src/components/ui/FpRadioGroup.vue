<template>
  <RadioButtonGroup v-bind="$attrs" :model-value="model" @update:model-value="emit">
    <template v-if="$slots.default">
      <slot />
    </template>
    <RadioButton
      v-for="opt in options"
      v-else
      :key="String(opt[optionValue ?? 'value'])"
      :value="opt[optionValue ?? 'value']"
      :input-id="`${uid}-${String(opt[optionValue ?? 'value'])}`"
      :label="opt[optionLabel ?? 'label']"
    />
  </RadioButtonGroup>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import RadioButtonGroup from 'openvue/radiobuttongroup'
import RadioButton from 'openvue/radiobutton'

withDefaults(
  defineProps<{
    options?: Array<Record<string, unknown>>
    optionLabel?: string
    optionValue?: string
  }>(),
  { options: () => [], optionLabel: 'label', optionValue: 'value' },
)

const model = defineModel<string>()
const uid = useId()

function emit(v: string | undefined) {
  model.value = v ?? ''
}
</script>
