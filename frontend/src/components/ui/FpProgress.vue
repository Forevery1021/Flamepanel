<template>
  <ProgressBar v-bind="$attrs" :value="value" />
</template>

<script setup lang="ts">
import ProgressBar from 'openvue/progressbar'

withDefaults(
  defineProps<{
    value?: number
  }>(),
  { value: 0 },
)
</script>
