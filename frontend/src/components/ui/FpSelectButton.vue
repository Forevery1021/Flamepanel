<template>
  <SelectButton v-bind="$attrs" :model-value="model" @update:model-value="emit" />
</template>

<script setup lang="ts">
import SelectButton from 'openvue/selectbutton'

const model = defineModel<string>()
const emit = (v: string | undefined) => {
  model.value = v ?? ''
}
</script>
