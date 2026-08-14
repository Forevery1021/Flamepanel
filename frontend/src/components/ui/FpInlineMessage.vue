<template>
  <InlineMessage v-bind="$attrs" :severity="severity" class="fp-inline-message">
    <slot />
  </InlineMessage>
</template>

<script setup lang="ts">
import InlineMessage from 'openvue/inlinemessage'

withDefaults(
  defineProps<{
    severity?: 'info' | 'success' | 'warn' | 'error'
  }>(),
  { severity: 'info' },
)
</script>

<style scoped>
.fp-inline-message {
  font-family: var(--fp-font-sans);
}
</style>
