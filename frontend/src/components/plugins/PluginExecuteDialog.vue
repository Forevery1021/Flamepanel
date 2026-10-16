<template>
  <FpModal v-model="open" :header="`${t('plugin.execute')} — ${pluginId}`" size="lg">
    <div class="modal-form">
      <FpInput v-model="func" :label="t('plugin.function')" placeholder="run" />
      <FpTextarea v-model="input" :label="t('plugin.input')" :rows="3" class="w-full" />
      <div>
        <FpButton variant="primary" :loading="running" @click="run">{{ t('plugin.run') }}</FpButton>
      </div>
    </div>
    <template v-if="result || error">
      <FpDivider />
      <FpInlineMessage v-if="error" severity="error">{{ error }}</FpInlineMessage>
      <template v-if="result">
        <div class="section-title">
          {{ t('plugin.output') }}
          <span class="muted">{{ t('plugin.duration', { ms: result.execution_ms }) }}</span>
        </div>
        <pre class="exec-output">{{ result.output_string || result.output_hex }}</pre>
        <template v-if="result.logs.length">
          <div class="section-title">{{ t('plugin.logs') }}</div>
          <pre class="exec-output">{{ result.logs.join('\n') }}</pre>
        </template>
      </template>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { executePlugin } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
import type { PluginExecutionResponse } from '@/types'

const props = defineProps<{ pluginId: string }>()
const emit = defineEmits<{ executed: [] }>()
const open = defineModel<boolean>({ default: false })
const { t } = useI18n()

const func = ref('run')
const input = ref('')
const running = ref(false)
const result = ref<PluginExecutionResponse | null>(null)
const error = ref('')

watch(open, (v) => {
  if (v) {
    func.value = 'run'
    input.value = ''
    result.value = null
    error.value = ''
  }
})

async function run() {
  running.value = true
  result.value = null
  error.value = ''
  try {
    const res = await executePlugin(
      props.pluginId,
      func.value.trim() || 'run',
      input.value || undefined,
    )
    result.value = res.data
    emit('executed')
  } catch (e: unknown) {
    error.value = getErrorMessage(e, t('common.failed'))
  } finally {
    running.value = false
  }
}
</script>

<style scoped>
.section-title {
  font-weight: 600;
  margin: var(--fp-space-3) 0 var(--fp-space-2);
  color: var(--fp-text-primary);
  display: flex;
  gap: var(--fp-space-2);
  align-items: baseline;
}
.muted {
  font-weight: 400;
  font-size: 12px;
  color: var(--fp-text-muted);
}
.exec-output {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  font-size: 13px;
  font-family: var(--fp-font-mono);
  max-height: 300px;
  overflow: auto;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
