<template>
  <Textarea
    v-bind="$attrs"
    :model-value="model"
    :invalid="!!error || invalid"
    :aria-invalid="!!error || invalid || undefined"
    :aria-describedby="error ? `${uid}-error` : undefined"
    class="w-full"
    @update:model-value="emit"
  />
  <small v-if="error" :id="`${uid}-error`" class="fp-textarea-error">{{ error }}</small>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import Textarea from 'openvue/textarea'

withDefaults(
  defineProps<{
    error?: string
    invalid?: boolean
  }>(),
  { error: '', invalid: false },
)

const model = defineModel<string>()
const uid = useId()
const emit = (v: string | undefined) => {
  model.value = v
}
</script>

<style scoped>
.fp-textarea-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
  margin-top: 4px;
}
</style>
