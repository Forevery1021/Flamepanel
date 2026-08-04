<template>
  <div class="fp-field" :class="{ 'fp-field-invalid': !!error || invalid }">
    <FloatLabel v-if="label">
      <Select
        :id="uid"
        v-bind="$attrs"
        :model-value="model"
        :invalid="!!error || invalid"
        :show-clear="showClear"
        :filter="filter"
        class="w-full"
        @update:model-value="emit"
      />
      <label :for="uid">{{ label }}</label>
    </FloatLabel>
    <Select
      v-else
      v-bind="$attrs"
      :model-value="model"
      :invalid="!!error || invalid"
      :show-clear="showClear"
      :filter="filter"
      class="w-full"
      @update:model-value="emit"
    />
    <small v-if="error" class="fp-field-error">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import Select from 'openvue/select'
import FloatLabel from 'openvue/floatlabel'

withDefaults(
  defineProps<{
    label?: string
    error?: string
    invalid?: boolean
    showClear?: boolean
    filter?: boolean
    multiple?: boolean
  }>(),
  { label: '', error: '', invalid: false, showClear: false, filter: false, multiple: false },
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
.fp-field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
</style>
