<template>
  <Checkbox
    v-bind="$attrs"
    :model-value="model"
    :binary="binary"
    @update:model-value="emitModel"
    @change="onChange"
  />
</template>

<script setup lang="ts">
import Checkbox from 'openvue/checkbox'

withDefaults(
  defineProps<{
    binary?: boolean
  }>(),
  { binary: true },
)

const model = defineModel<boolean>({ default: false })
const emit = defineEmits<{ change: [value: boolean] }>()

function emitModel(v: boolean | undefined) {
  model.value = v ?? false
}

function onChange(v: unknown) {
  emit('change', Boolean(v))
}
</script>
