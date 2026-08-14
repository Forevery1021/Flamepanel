<template>
  <Chip v-bind="$attrs" :label="label" :removable="removable" class="fp-chip" />
</template>

<script setup lang="ts">
import Chip from 'openvue/chip'

withDefaults(
  defineProps<{
    label?: string
    removable?: boolean
  }>(),
  { label: '', removable: false },
)
</script>

<style scoped>
.fp-chip {
  font-family: var(--fp-font-sans);
}
</style>
