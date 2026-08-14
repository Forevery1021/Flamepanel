<template>
  <Column v-bind="$attrs" class="fp-column">
    <template v-for="(_, name) in $slots" #[name]="slotProps">
      <slot :name="name" v-bind="slotProps" />
    </template>
  </Column>
</template>

<script setup lang="ts">
import Column from 'openvue/column'

defineOptions({ inheritAttrs: false })
</script>
