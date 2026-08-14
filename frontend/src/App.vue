<template>
  <!-- P7 可靠性：渲染错误兜底，避免白屏 -->
  <FpStatePanel
    v-if="fatalError"
    class="app-error-panel"
    :title="t('common.failed')"
    :description="fatalError"
    :retryable="true"
    @retry="recover"
  />
  <template v-else>
    <router-view />
  </template>
  <ConfirmDialog />
  <Toast />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { onErrorCaptured } from 'vue'
import { useI18n } from 'vue-i18n'
import Toast from 'openvue/toast'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import { reportError } from '@/utils/monitor'

const { t } = useI18n()
const fatalError = ref<string | null>(null)

onErrorCaptured((err, _instance, info) => {
  // 结构化上报（供监控/排障）
  reportError(err, { source: 'render', context: info })
  // 渲染错误兜底：展示错误面板而非白屏
  fatalError.value = err instanceof Error ? err.message : String(err)
  return false // 阻止错误继续向上传播
})

function recover() {
  fatalError.value = null
}
</script>

<style scoped>
.app-error-panel {
  padding: var(--fp-space-8);
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
