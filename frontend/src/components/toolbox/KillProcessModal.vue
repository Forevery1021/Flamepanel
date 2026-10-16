<template>
  <FpModal
    :model-value="open"
    :header="t('processes.kill.title')"
    size="md"
    @update:model-value="onToggle"
  >
    <div v-if="process" class="kill-body">
      <dl class="proc-meta">
        <dt>PID</dt>
        <dd class="mono">{{ process.pid }}</dd>
        <dt>{{ t('processes.col.name') }}</dt>
        <dd class="proc-name">{{ process.name }}</dd>
        <dt>{{ t('processes.col.user') }}</dt>
        <dd>{{ process.user ?? '—' }}</dd>
        <dt>{{ t('processes.col.command') }}</dt>
        <dd class="mono proc-cmd">{{ process.command }}</dd>
      </dl>

      <fieldset class="signal-set">
        <legend class="signal-legend">{{ t('processes.kill.signal') }}</legend>
        <FpRadioGroup v-model="signal" class="signal-group">
          <div v-for="s in PROCESS_SIGNALS" :key="s" class="signal-option">
            <FpRadioOption :value="s" :input-id="`${uid}-${s}`" />
            <label :for="`${uid}-${s}`" class="signal-label">
              <span class="signal-name mono">SIG{{ s }}</span>
              <span class="signal-desc">{{ t(signalI18nKey(s)) }}</span>
            </label>
          </div>
        </FpRadioGroup>
      </fieldset>

      <FpInlineMessage :severity="signal === 'KILL' ? 'warn' : 'info'">
        {{ signal === 'KILL' ? t('processes.kill.killWarning') : t('processes.kill.hint') }}
      </FpInlineMessage>
    </div>

    <template #footer>
      <FpButton variant="ghost" @click="onToggle(false)">{{ t('common.cancel') }}</FpButton>
      <FpButton
        variant="danger"
        icon="oi oi-stop-circle"
        :loading="loading"
        :disabled="loading || !process"
        class="kill-confirm"
        @click="onConfirm"
      >
        {{ t('processes.kill.confirm', { signal: `SIG${signal}` }) }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, useId, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpRadioGroup from '@/components/ui/FpRadioGroup.vue'
import FpRadioOption from '@/components/ui/FpRadioOption.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { PROCESS_SIGNALS, type ProcessRow, type ProcessSignal } from '@/api/toolbox'
import { signalI18nKey } from '@/utils/processes'

/**
 * S-12 结束进程确认弹窗：展示目标进程 + 选择信号（缺省 SIGTERM）。
 * 只负责收集意图；真正的请求 / 错误提示由页面处理。
 */
const props = defineProps<{
  open: boolean
  process: ProcessRow | null
  loading?: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: [signal: ProcessSignal]
}>()

const { t } = useI18n()
const uid = useId()
const signal = ref<ProcessSignal>('TERM')

// 每次打开都回到最安全的 SIGTERM，避免上一次选的 SIGKILL 被\"记住\"
watch(
  () => props.open,
  (open) => {
    if (open) signal.value = 'TERM'
  },
)

function onToggle(value: boolean) {
  emit('update:open', value)
}

function onConfirm() {
  emit('confirm', signal.value)
}
</script>

<style scoped>
.kill-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.proc-meta {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--fp-space-1) var(--fp-space-3);
  margin: 0;
  font-size: 13px;
}
.proc-meta dt {
  color: var(--fp-text-muted);
}
.proc-meta dd {
  margin: 0;
  color: var(--fp-text-primary);
}
.proc-name {
  font-weight: 600;
}
.proc-cmd {
  font-size: 12px;
  word-break: break-all;
  max-height: 6em;
  overflow: auto;
  color: var(--fp-text-code);
}
.signal-set {
  border: 0;
  margin: 0;
  padding: 0;
}
.signal-legend {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: var(--fp-space-2);
  color: var(--fp-text-primary);
}
.signal-group {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.signal-option {
  display: flex;
  align-items: flex-start;
  gap: var(--fp-space-2);
}
.signal-label {
  display: flex;
  flex-direction: column;
  cursor: pointer;
  font-size: 13px;
}
.signal-name {
  font-weight: 600;
  color: var(--fp-text-primary);
}
.signal-desc {
  font-size: 12px;
  color: var(--fp-text-muted);
}
</style>
