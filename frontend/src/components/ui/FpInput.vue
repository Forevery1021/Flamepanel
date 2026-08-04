<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <FloatLabel v-if="label">
      <InputText
        :id="uid"
        v-bind="$attrs"
        :model-value="model"
        :invalid="!!error || invalid"
        class="w-full"
        :class="{ 'text-right': align === 'right' }"
        @update:model-value="emit"
      />
      <label :for="uid">{{ label }}</label>
    </FloatLabel>
    <InputText
      v-else
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      class="w-full"
      :class="{ 'text-right': align === 'right' }"
      @update:model-value="emit"
    />
    <small v-if="error" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import InputText from 'openvue/inputtext'
import FloatLabel from 'openvue/floatlabel'

withDefaults(
  defineProps<{
    label?: string
    error?: string
    invalid?: boolean
    align?: 'left' | 'right'
  }>(),
  { label: '', error: '', invalid: false, align: 'left' },
)

const model = defineModel<string>()
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
.fp-field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
