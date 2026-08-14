<template>
  <div class="fp-switch">
    <ToggleSwitch
      :model-value="model"
      :disabled="disabled"
      @update:model-value="emit"
    />
  </div>
</template>

<script setup lang="ts">
import ToggleSwitch from 'openvue/toggleswitch'

withDefaults(
  defineProps<{
    disabled?: boolean
  }>(),
  { disabled: false },
)

const model = defineModel<boolean>({ default: false })

function emit(v: boolean | undefined) {
  model.value = !!v
}
</script>

<style scoped>
.fp-switch {
  display: inline-flex;
  align-items: center;
}
</style>
