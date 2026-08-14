<template>
  <LayoutContent :title="t('nav.tasks')" reload @reload="invalidate">
    <template #toolbar>
      <div class="toolbar-left">
        <FpSelect
          v-model="stateFilter"
          :options="stateOptions"
          option-label="label"
          option-value="value"
          show-clear
          class="toolbar-filter"
          style="width: 160px"
        />
      </div>
      <FpButton variant="ghost" icon="oi oi-trash" @click="onPrune">
        {{ t('task.prune') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable
        :rows="tasks"
        :loading="loading"
        :empty-text="t('common.noData')"
      >
        <FpColumn field="id" :header="t('task.id')" style="width: 80px" />
        <FpColumn :header="t('task.kind')" style="width: 120px">
          <template #body="{ data }">
            <FpTag :severity="kindTag(data.kind)" :value="kindLabel(data.kind)" />
          </template>
        </FpColumn>
        <FpColumn field="name" :header="t('task.name')" style="width: 200px" />
        <FpColumn :header="t('task.state')" style="width: 100px">
          <template #body="{ data }">
            <FpTag :severity="stateTag(data.state)" :value="stateLabel(data.state)" />
          </template>
        </FpColumn>
        <FpColumn :header="t('task.progress')" style="width: 200px">
          <template #body="{ data }">
            <div class="progress-cell">
              <FpProgress :value="data.progress" :style="`height: 6px; flex: 1`" />
              <span class="progress-text">{{ data.progress }}%</span>
            </div>
          </template>
        </FpColumn>
        <FpColumn :header="t('task.message')">
          <template #body="{ data }">
            <span class="message-cell">{{ data.message || '-' }}</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('task.updatedAt')" style="width: 170px">
          <template #body="{ data }">{{ formatTime(data.updated_at) }}</template>
        </FpColumn>
        <FpColumn :header="t('common.colActions')" style="width: 100px" frozen>
          <template #body="{ data }">
            <FpButton
              v-if="data.state === 'pending' || data.state === 'running'"
              variant="ghost"
              icon="oi oi-x"
              @click="onCancel(data)"
            >
              {{ t('task.cancel') }}
            </FpButton>
          </template>
        </FpColumn>
      </FpTable>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpProgress from '@/components/ui/FpProgress.vue'
import { listTasks, cancelTask, pruneTasks, type TaskRecord, type TaskState } from '@/api/tasks'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'

const { t } = useI18n()
const toast = useFpToast()

const queryClient = useQueryCacheClient()
const stateFilter = ref<string>()

const stateOptions = [
  { label: t('task.statePending'), value: 'pending' },
  { label: t('task.stateRunning'), value: 'running' },
  { label: t('task.stateSuccess'), value: 'success' },
  { label: t('task.stateFailed'), value: 'failed' },
  { label: t('task.stateCancelled'), value: 'cancelled' },
]

function stateLabel(s: TaskState) {
  const map: Record<TaskState, string> = {
    pending: t('task.statePending'),
    running: t('task.stateRunning'),
    success: t('task.stateSuccess'),
    failed: t('task.stateFailed'),
    cancelled: t('task.stateCancelled'),
  }
  return map[s] || s
}

type FpTagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

function stateTag(s: TaskState): FpTagSeverity {
  const map: Record<TaskState, FpTagSeverity> = {
    pending: 'info',
    running: 'warning',
    success: 'success',
    failed: 'danger',
    cancelled: 'neutral',
  }
  return map[s] || 'info'
}

function kindLabel(k: string) {
  const map: Record<string, string> = {
    install: t('task.kindInstall'),
    engine_switch: t('task.kindEngineSwitch'),
    batch_node: t('task.kindBatchNode'),
    generic: t('task.kindGeneric'),
  }
  return map[k] || k
}

function kindTag(k: string): FpTagSeverity {
  const map: Record<string, FpTagSeverity> = {
    install: 'info',
    engine_switch: 'warning',
    batch_node: 'info',
    generic: 'neutral',
  }
  return map[k] || 'info'
}

function formatTime(iso: string) {
  if (!iso) return '-'
  const d = new Date(iso)
  return d.toLocaleString()
}

// P3-A：任务列表走统一数据获取层 useApiQuery（stateFilter 变化自动重新拉取）
const tasksQuery = useApiQuery<{ tasks: TaskRecord[] }>(
  () => queryKeys.tasks.list(stateFilter.value),
  async () => {
    const res = await listTasks(stateFilter.value as TaskState | undefined)
    return { data: res.data }
  },
)
const tasks = computed<TaskRecord[]>(() => tasksQuery.data.value?.tasks ?? [])
const loading = tasksQuery.loading

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.tasks.all })
}

async function onCancel(task: TaskRecord) {
  try {
    await cancelTask(task.id)
    toast.success(t('task.cancelled'))
    invalidate()
  } catch (e) {
    toast.error(e, t('task.cancelFailed'))
  }
}

async function onPrune() {
  try {
    const res = await pruneTasks()
    toast.success(t('task.pruned', { count: res.data.pruned }))
    invalidate()
  } catch (e) {
    toast.error(e, t('task.pruneFailed'))
  }
}

</script>

<style scoped>
.toolbar-left {
  display: flex;
  gap: 8px;
  align-items: center;
}
.toolbar-filter {
  max-width: 220px;
}
.progress-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}
.progress-text {
  font-size: 12px;
  color: var(--fp-text-muted, #8a8f98);
  min-width: 36px;
}
.message-cell {
  color: var(--fp-text-muted, #8a8f98);
}
</style>
