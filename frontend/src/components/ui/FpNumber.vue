<template>
  <div class="fp-number" :class="{ 'fp-field-invalid': !!error || invalid }">
    <FloatLabel v-if="label">
      <InputNumber
        :id="uid"
        v-bind="$attrs"
        :model-value="model"
        :invalid="!!error || invalid"
        class="w-full"
        @update:model-value="emit"
      />
      <label :for="uid">{{ label }}</label>
    </FloatLabel>
    <InputNumber
      v-else
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
import FloatLabel from 'openvue/floatlabel'

withDefaults(
  defineProps<{
    label?: string
    error?: string
    invalid?: boolean
  }>(),
  { label: '', error: '', invalid: false },
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
.fp-field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
