<template>
  <Skeleton v-bind="$attrs" class="fp-skeleton" />
</template>

<script setup lang="ts">
import Skeleton from 'openvue/skeleton'
</script>

<style scoped>
.fp-skeleton {
  border-radius: var(--fp-radius-sm);
}
</style>
