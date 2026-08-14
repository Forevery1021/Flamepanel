<template>
  <RadioButton v-bind="$attrs" :value="value" :input-id="inputId" />
</template>

<script setup lang="ts">
import RadioButton from 'openvue/radiobutton'

withDefaults(
  defineProps<{
    value?: string | number | boolean
    inputId?: string
  }>(),
  { value: '', inputId: '' },
)
</script>
