<template>
  <FpModal v-model="visible" :header="t('node.batchExec')" size="lg">
    <div class="modal-form">
      <div class="text-muted text-xs">{{ t('node.selectNodesHint') }}</div>
      <div class="batch-node-list">
        <label v-for="n in nodes" :key="n.id" class="batch-node-item">
          <input v-model="selectedIds" type="checkbox" :value="n.id" />
          <span>{{ n.name }} ({{ n.ip_address }})</span>
        </label>
      </div>
      <FpInput
        v-model="command"
        :label="t('node.command')"
        :placeholder="t('node.commandPlaceholder')"
      />
      <div v-if="result.length" class="batch-result">
        <div v-for="item in result" :key="item.node_id" class="batch-result-item">
          <div class="batch-result-head">
            <span class="text-sm">[{{ item.node_name }}]</span>
            <FpTag
              :severity="item.success ? 'success' : 'danger'"
              :value="item.success ? 'OK' : 'ERR'"
            />
          </div>
          <pre class="exec-output">{{ item.result?.output || item.result?.error || '—' }}</pre>
        </div>
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="loading" @click="emit('run')">
        {{ t('node.runBatch') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { BatchExecItem } from '@/types'
import type { EnrichedNode } from '@/utils/nodeFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 批量命令弹窗（Doc/23 §3.3 T2-4 视图拆分第七站 · Stage5 批量命令）。
 *
 * 三处不能丢的语义：
 *
 * 1. **节点清单来自**当前页的列表数据**（父级传入），勾选后按 id 上抛——
 *    跨页批量本就超出单页上下文，强行做会需要额外一次全量拉取（原实现口径）；
 * 2. **结果逐节点渲染**，且输出优先取 `result.output`、其次 `result.error`、
 *    最后 `—`：三态在同一个 `<pre>` 里表达，用户不必分辨「这次是哪种失败」；
 * 3. **勾选状态是本组件的本地态**（`v-model:selectedIds`），打开时由数据层
 *    预置「全选当前页」——「批量」的默认预期就是这一批一起跑。
 *
 * 用原生 `<input type="checkbox">` 而非 FpCheckbox：批量勾选是**纯集合操作**
 * （无中间态、无二进制/单选语义），原生元素在密集列表里更轻，且原实现即如此。
 */
withDefaults(
  defineProps<{
    /** 候选节点（当前页） */
    nodes?: EnrichedNode[]
    /** 执行态 */
    loading?: boolean
    /** 逐节点结果（空数组 = 还没跑过） */
    result?: BatchExecItem[]
  }>(),
  { nodes: () => [], loading: false, result: () => [] },
)

const visible = defineModel<boolean>({ required: true })
const command = defineModel<string>('command', { default: '' })
const selectedIds = defineModel<number[]>('selectedIds', { default: () => [] })

const emit = defineEmits<{ run: [] }>()

const { t } = useI18n()
</script>

<style scoped>
.batch-node-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-3);
}
.batch-node-item {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 13px;
}
.batch-result {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}
.batch-result-item {
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-2);
}
.batch-result-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
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
