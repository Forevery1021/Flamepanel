<template>
  <!-- Catch fatal layout/router errors without leaving a blank screen. -->
  <FpErrorBoundary class="app-root">
    <router-view />
  </FpErrorBoundary>
  <ConfirmDialog />
  <Toast />
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import Toast from 'openvue/toast'
import FpErrorBoundary from '@/components/ui/FpErrorBoundary.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useSafeText } from '@/composables/useSafeText'
import { fallbackText } from '@/utils/fallbackText'
import { onNotice } from '@/utils/notice'

/**
 * App catches fatal layout/router errors. Layout's page boundary preserves
 * navigation when only a page fails (see components/Layout.vue).
 * Route guards run outside the component tree and deliver notices here.
 */
const tt = useSafeText()
const toast = useFpToast()
let unsubscribe: (() => void) | null = null

onMounted(() => {
  unsubscribe = onNotice((message) => {
    // Fall back to readable text if locale loading races; never display raw keys.
    const detail = tt(message.key, fallbackText().failed)
    toast.show({ severity: message.severity, summary: '', detail, life: 3500 })
  })
})

onBeforeUnmount(() => {
  unsubscribe?.()
  unsubscribe = null
})
</script>

<style scoped>
/* Fatal errors have no navigation shell, so fill the viewport. */
.app-root :deep(.fp-eb) {
  min-height: 100vh;
}
</style>
