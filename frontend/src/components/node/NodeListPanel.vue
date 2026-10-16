<template>
  <div class="panel">
    <div class="toolbar">
      <span class="text-muted text-xs">{{ t('node.liveHint') }}</span>
    </div>
    <FpTable
      :rows="rows"
      :loading="loading"
      :first="first"
      :empty-text="t('common.noData')"
      :empty-desc="$slots.emptyAction ? t('pageDesc.emptyHint') : ''"
    >
      <template v-if="$slots.emptyAction" #emptyAction>
        <slot name="emptyAction" />
      </template>
      <FpColumn field="id" :header="t('node.id')" style="width: 70px" />
      <FpColumn field="name" :header="t('node.name')" />
      <FpColumn :header="t('node.hostname')">
        <template #body="{ data }">
          <span v-tooltip="data.hostname" class="cell-truncate">{{ data.hostname }}</span>
        </template>
      </FpColumn>
      <FpColumn field="ip_address" :header="t('node.ip')" />
      <FpColumn :header="t('node.status')" style="width: 150px">
        <template #body="{ data }">
          <div class="status-cell">
            <FpTag
              :severity="data.online ? 'success' : 'danger'"
              :dot="data.online"
              :value="data.online ? t('dashboard.online') : t('dashboard.offline')"
            />
            <FpTag v-if="data.edge?.connected" v-tooltip="t('node.edgeHint')" severity="info" :value="t('node.edge')" />
          </div>
        </template>
      </FpColumn>
      <FpColumn :header="t('node.metrics')">
        <template #body="{ data }">
          <div v-if="data.metrics" class="metrics-row">
            <div class="metrics-bar">
              <div class="metrics-bar-fill" :style="cpuBarStyle(data.metrics)" />
            </div>
            <span class="text-xs text-muted">{{ nodeMetricsSummary(data.metrics) }}</span>
          </div>
          <span v-else class="text-muted text-xs">—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('node.lastHeartbeat')" style="width: 150px">
        <template #body="{ data }">
          <span v-if="data.last_heartbeat_at" class="text-xs">{{ data.last_heartbeat_at }}</span>
          <span v-else class="text-muted text-xs">—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.operation')" style="width: 130px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton variant="link" @click="emit('monitor', data)">{{ t('node.monitor') }}</FpButton>
            <FpButton v-permission="{ perm: 'node:update', mode: 'view' }" variant="link" @click="emit('edit', data)">{{ t('common.edit') }}</FpButton>
            <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="link" @click="emit('exec', data)">{{ t('node.remoteExec') }}</FpButton>
            <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="link" @click="emit('files', data)">{{ t('node.remoteFiles') }}</FpButton>
            <FpButton
              v-if="data.edge?.connected && data.edge.terminal"
              v-permission="{ perm: 'node:execute', mode: 'view' }"
              variant="link"
              @click="emit('terminal', data)"
            >
              {{ t('node.terminal') }}
            </FpButton>
            <FpButton v-permission="{ perm: 'node:delete', mode: 'view' }" variant="link" @click="emit('remove', data)">{{ t('common.delete') }}</FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
    <FpPagination
      v-if="total > pageSize"
      :first="first"
      :rows="pageSize"
      :total="total"
      :rows-per-page-options="[20, 50, 100]"
      @update:first="emit('update:first', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { EnrichedNode } from '@/utils/nodeFormat'
import { cpuBarStyle, nodeMetricsSummary } from '@/utils/nodeFormat'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpPagination from '@/components/ui/FpPagination.vue'

/**
 * 节点列表（Doc/23 §3.3 T2-4 视图拆分第七站）。
 *
 * **纯展示 + 事件上抛**：本组件不持有任何请求状态，四类行内操作（编辑 / 远程命令 /
 * 远程文件 / 删除）与分页翻页全部交由父级编排。列宽、冻结操作列、`v-permission`
 * 权限项沿用原实现（拆分不改视觉与授权契约）。
 *
 * 三处展示口径下沉到 `utils/nodeFormat` 而非在模板里内联：
 * - `cpuBarStyle`：宽度取整 + 阈值取色（与仪表盘 `usageColor` 同源）；
 * - `nodeMetricsSummary`：三个维度一律一位小数；
 * - 指标缺失渲染 `—`（Agent 从没上报过 / 上报的是脏 JSON）。
 *
 * 心跳列同样缺失渲染 `—`，**不做「多久之前」的相对时间**：后端下发的就是
 * 时间戳文本，把它改写成相对时间会让用户无法与 Agent 日志对时。
 */
withDefaults(
  defineProps<{
    rows: EnrichedNode[]
    loading?: boolean
    total?: number
    first?: number
    pageSize?: number
  }>(),
  { loading: false, total: 0, first: 0, pageSize: 20 },
)

const emit = defineEmits<{
  edit: [row: EnrichedNode]
  exec: [row: EnrichedNode]
  files: [row: EnrichedNode]
  /** Agent Edge：打开该节点的交互式远程终端（仅 Edge 在线且 Agent 开启终端时出现） */
  terminal: [row: EnrichedNode]
  /** Doc/30 M-P：跳转仪表盘查看该节点实时指标（node:read 即可，与列表同权限） */
  monitor: [row: EnrichedNode]
  remove: [row: EnrichedNode]
  'update:first': [first: number]
}>()

const { t } = useI18n()
</script>

<style scoped>
.toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.status-cell {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-1);
  align-items: center;
}
.cell-truncate {
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.metrics-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.metrics-bar {
  width: 200px;
  height: 6px;
  border-radius: 3px;
  background: var(--fp-bg-hover);
  overflow: hidden;
}
.metrics-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 200ms var(--fp-ease-out);
}
</style>
