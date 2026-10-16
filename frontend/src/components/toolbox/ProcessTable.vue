<template>
  <FpTable
    :rows="rows"
    :loading="false"
    :first="first"
    :rows-per-page="pageSize"
    :rows-per-page-options="[20, 50, 100]"
    :empty-text="t('processes.empty')"
    data-key="pid"
    class="process-table"
    @update:first="emit('update:first', $event)"
  >
    <FpColumn field="pid" header="PID" style="width: 84px">
      <template #body="{ data: row }">
        <span class="mono">{{ row.pid }}</span>
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.name')" style="min-width: 160px">
      <template #body="{ data: row }">
        <div class="name-cell">
          <span class="proc-name" :title="row.name">{{ row.name }}</span>
          <FpTag v-if="row.pid === panelPid" severity="info" :value="t('processes.panelTag')" />
          <FpTag v-else-if="row.kernel_thread" :value="t('processes.kernelTag')" />
        </div>
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.user')" style="width: 120px">
      <template #body="{ data: row }">{{ row.user ?? row.uid ?? '—' }}</template>
    </FpColumn>
    <FpColumn :header="t('processes.col.cpu')" style="width: 90px">
      <template #body="{ data: row }">
        <span class="mono num" :class="`tone-${cpuTone(row.cpu)}`">{{ formatCpu(row.cpu) }}%</span>
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.memory')" style="width: 130px">
      <template #body="{ data: row }">
        <div class="mem-cell">
          <span class="mono num">{{ formatSize(row.memory_bytes) }}</span>
          <span class="mem-bar" aria-hidden="true">
            <span
              class="mem-bar__fill"
              :style="{
                width: `${memoryShare(row.memory_bytes, totalMemoryBytes)}%`,
              }"
            />
          </span>
        </div>
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.status')" style="width: 110px">
      <template #body="{ data: row }">
        <FpTag
          :severity="stateTone(normalizeState(row.status))"
          :value="t(`processes.state.${normalizeState(row.status)}`)"
          :title="row.status"
        />
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.runTime')" style="width: 100px">
      <template #body="{ data: row }">
        <span class="mono num" :title="startedTitle(row.started_at)">{{
          formatRunTime(row.started_at, now)
        }}</span>
      </template>
    </FpColumn>
    <FpColumn :header="t('processes.col.command')" style="min-width: 240px">
      <template #body="{ data: row }">
        <code class="cmd" :title="row.command">{{ row.command }}</code>
      </template>
    </FpColumn>
    <FpColumn :header="t('common.colActions')" style="width: 96px" frozen align-frozen="right">
      <template #body="{ data: row }">
        <FpButton
          v-permission="{ perm: 'toolbox:execute', mode: 'view' }"
          variant="link"
          class="kill-btn"
          :disabled="!!row.protected"
          :title="row.protected ? t(protectedI18nKey(row.protected)) : ''"
          @click="emit('kill', row)"
        >
          {{ t('processes.kill.action') }}
        </FpButton>
      </template>
    </FpColumn>
  </FpTable>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import FpButton from '@/components/ui/FpButton.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpTag from '@/components/ui/FpTag.vue'
import type { ProcessRow } from '@/api/toolbox'
import { formatSize } from '@/utils/format'
import {
  cpuTone,
  formatCpu,
  formatRunTime,
  memoryShare,
  normalizeState,
  protectedI18nKey,
  stateTone,
} from '@/utils/processes'

/**
 * S-12 进程表格（纯展示）：PID / 名称（面板 / 内核线程标记）/ 用户 / CPU / 内存占比条 /
 * 状态 / 运行时长 / 脱敏命令行 / 结束按钮。受保护进程的按钮禁用并以 title 提示原因。
 */
defineProps<{
  rows: ProcessRow[]
  panelPid: number | null
  totalMemoryBytes: number
  /** 计算运行时长的参照时间（ms），由父级定时推进 */
  now: number
  first: number
}>()

const emit = defineEmits<{
  kill: [row: ProcessRow]
  'update:first': [value: number]
}>()

const { t } = useI18n()
const pageSize = 20

function startedTitle(startedAt: number): string {
  return startedAt ? new Date(startedAt * 1000).toLocaleString() : ''
}
</script>

<style scoped>
.name-cell {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  min-width: 0;
}
.proc-name {
  font-weight: 600;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.num {
  font-variant-numeric: tabular-nums;
}
.tone-danger {
  color: var(--fp-danger);
  font-weight: 600;
}
.tone-warning {
  color: var(--fp-warning);
  font-weight: 600;
}
.tone-neutral {
  color: var(--fp-text-secondary);
}
.mem-cell {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.mem-bar {
  display: block;
  height: 3px;
  border-radius: 2px;
  background: var(--fp-bg-subtle);
  overflow: hidden;
}
.mem-bar__fill {
  display: block;
  height: 100%;
  background: var(--fp-brand);
  min-width: 1px;
}
.cmd {
  display: block;
  max-width: 520px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--fp-font-mono);
  font-size: 12px;
  color: var(--fp-text-code);
}
</style>
