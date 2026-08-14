<template>
  <Divider v-bind="$attrs" class="fp-divider" />
</template>

<script setup lang="ts">
import Divider from 'openvue/divider'
</script>

<style scoped>
.fp-divider {
  border-color: var(--fp-border);
}
</style>
