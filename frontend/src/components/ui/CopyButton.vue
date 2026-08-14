<template>
  <FpButton
    variant="ghost"
    :icon="copied ? 'oi oi-check' : 'oi oi-copy'"
    :aria-label="t('common.copy')"
    :title="t('common.copy')"
    @click="copy"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpButton from './FpButton.vue'
import { useFpToast } from './FpToast'

const props = defineProps<{ text?: string }>()
const { t } = useI18n()
const toast = useFpToast()

const copied = ref(false)

async function copy() {
  try {
    await navigator.clipboard.writeText(props.text ?? '')
    copied.value = true
    toast.success(t('common.copySuccess'))
    setTimeout(() => (copied.value = false), 1500)
  } catch {
    toast.error(t('common.copyFailed'))
  }
}
</script>
