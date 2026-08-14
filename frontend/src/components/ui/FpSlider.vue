<template>
  <Slider v-bind="$attrs" :model-value="model" @update:model-value="emit" />
</template>

<script setup lang="ts">
import Slider from 'openvue/slider'

const model = defineModel<number>({ default: 0 })
const emit = (v: number | number[] | undefined) => {
  model.value = Array.isArray(v) ? (v[0] ?? 0) : (v ?? 0)
}
</script>
