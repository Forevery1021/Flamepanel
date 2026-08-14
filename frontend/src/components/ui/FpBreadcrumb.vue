<template>
  <div class="fp-breadcrumb">
    <Breadcrumb v-bind="$attrs" :model="items" />
  </div>
</template>

<script setup lang="ts">
import Breadcrumb from 'openvue/breadcrumb'
import type { MenuItem } from 'openvue/menuitem'

withDefaults(
  defineProps<{
    items?: MenuItem[]
  }>(),
  { items: () => [] },
)
</script>

<style scoped>
.fp-breadcrumb {
  display: flex;
  align-items: center;
}
</style>
