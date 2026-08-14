<template>
  <Button
    v-bind="$attrs"
    :label="label"
    :text="text"
    :size="size"
    class="fp-button-link"
  >
    <slot />
  </Button>
</template>

<script setup lang="ts">
import Button from 'openvue/button'

withDefaults(
  defineProps<{
    label?: string
    text?: boolean
    size?: 'small' | 'large'
  }>(),
  { label: '', text: true, size: 'small' },
)
</script>

<style scoped>
.fp-button-link {
  font-family: var(--fp-font-sans);
}
</style>
