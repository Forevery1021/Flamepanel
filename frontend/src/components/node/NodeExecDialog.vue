<template>
  <FpModal v-model="visible" :header="t('node.remoteExec')" size="lg">
    <div class="modal-form">
      <div v-if="nodeName" class="text-muted text-xs">{{ t('node.name') }}: {{ nodeName }}</div>
      <FpInput
        v-model="command"
        :label="t('node.command')"
        :placeholder="t('node.commandPlaceholder')"
        :error="error"
      />
      <div v-if="result" class="exec-result">
        <div class="exec-result-head">
          <span class="text-muted text-xs">
            {{ t('node.exitCode') }}: {{ result.exit_code }} · {{ t('node.duration') }}:
            {{ result.duration_ms }}ms
          </span>
        </div>
        <pre class="exec-output">{{ result.output || '—' }}</pre>
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="loading" @click="emit('run', command)">
        {{ t('node.run') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { RemoteExecResult } from '@/types'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 远程命令弹窗（Doc/23 §3.3 T2-4 视图拆分第七站 · Stage5 远程命令）。
 *
 * 三处展示口径：
 *
 * 1. **结果区在弹窗内常驻，不在 toast 里**：远程执行失败（Agent 不可达）与命令
 *    跑完但退出码非 0 都需要看到「输出」——数据层把失败也写进 `result`
 *    （`exit_code: -1`），本组件只要有结果就渲染结果区，不区分成败；
 * 2. **输入错时字段旁红字**（`error` prop），不发请求前就给反馈；
 * 3. **命令由本组件持有、执行由父级发起**（`emit('run')`）：这样「点执行」时
 *    看到的就是输入框里的那份文本，不存在父级再存一份副本的可能。
 *
 * ⚠️ 目标节点的**选择**不在此弹窗内（工具栏进入时 `nodeName` 为空，仅行内进入
 * 才带节点）——这是原实现的行为，本轮拆分**原样保留**（见 PR「后续建议」：
 * 工具栏入口实际没有选节点的入口，点执行会被数据层拦下提示「请至少选择一个节点」）。
 */
withDefaults(
  defineProps<{
    /** 目标节点名（工具栏进入为空，行内进入为节点名） */
    nodeName?: string
    /** 执行态 */
    loading?: boolean
    /** 执行结果（成功或失败都走这里；null = 未执行） */
    result?: RemoteExecResult | null
    /** 命令字段错误（本地校验或后端拒绝） */
    error?: string
  }>(),
  { nodeName: '', loading: false, result: null, error: '' },
)

const visible = defineModel<boolean>({ required: true })
const command = defineModel<string>('command', { default: '' })

const emit = defineEmits<{ run: [command: string] }>()

const { t } = useI18n()
</script>

<style scoped>
.exec-result {
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-3);
  background: var(--fp-bg-subtle);
}
.exec-result-head {
  margin-bottom: 6px;
}
.exec-output {
  margin: 0;
  padding: var(--fp-space-3);
  background: var(--fp-bg-hover);
  border-radius: var(--fp-radius-sm);
  font-family: var(--fp-font-mono, monospace);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 240px;
  overflow-y: auto;
}
</style>
