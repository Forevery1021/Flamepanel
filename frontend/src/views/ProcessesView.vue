<template>
  <LayoutContent
    :title="t('processes.title')"
    :description="t('pageDesc.processes')"
    :summary="summary"
    reload
    @reload="refresh"
  >
    <template #actions>
      <FpSwitch v-model="autoRefresh" :label="t('processes.autoRefresh')" class="auto-refresh" />
    </template>

    <template #toolbar>
      <div class="toolbar-left">
        <FpInput
          v-model="searchText"
          :placeholder="t('processes.searchPlaceholder')"
          :aria-label="t('processes.searchPlaceholder')"
          class="toolbar-search"
        />
        <FpSelect
          v-model="userFilter"
          :options="userSelectOptions"
          option-label="label"
          option-value="value"
          :placeholder="t('processes.allUsers')"
          show-clear
          filter
          class="toolbar-filter"
        />
        <FpSelect
          v-model="sort"
          :options="sortOptions"
          option-label="label"
          option-value="value"
          class="toolbar-filter"
        />
        <FpSelect
          v-model="order"
          :options="orderOptions"
          option-label="label"
          option-value="value"
          class="toolbar-order"
        />
      </div>
      <div class="toolbar-right">
        <FpSwitch v-model="includeKernel" :label="t('processes.includeKernel')" />
        <FpSelect
          v-model="limit"
          :options="limitOptions"
          option-label="label"
          option-value="value"
          class="toolbar-order"
        />
      </div>
    </template>

    <FpStatePanel
      :loading="loading && !data"
      :error="loadError"
      retryable
      :title="loadError ? t('processes.loadFailed') : ''"
      @retry="refresh"
    >
      <div class="panel">
        <p v-if="truncated" class="truncated-hint">
          {{ t('processes.truncated', { shown: rows.length, matched: data?.matched ?? 0 }) }}
        </p>
        <ProcessTable
          v-model:first="first"
          :rows="rows"
          :panel-pid="data?.panel_pid ?? null"
          :total-memory-bytes="data?.total_memory_bytes ?? 0"
          :now="now"
          @kill="openKill"
        />
      </div>
    </FpStatePanel>

    <KillProcessModal
      v-model:open="killOpen"
      :process="killTarget"
      :loading="killing"
      @confirm="onConfirmKill"
    />
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { refDebounced, useIntervalFn } from '@vueuse/core'

import LayoutContent, { type LayoutSummaryItem } from '@/components/ui/LayoutContent.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useFpToast } from '@/components/ui/FpToast'
import KillProcessModal from '@/components/toolbox/KillProcessModal.vue'
import ProcessTable from '@/components/toolbox/ProcessTable.vue'
import { useProcesses } from '@/composables/useProcesses'
import {
  PROCESS_SORTS,
  type ProcessRow,
  type ProcessSignal,
  type ProcessSortField,
} from '@/api/toolbox'
import { formatSize } from '@/utils/format'
import { killErrorKey, userOptions } from '@/utils/processes'

/** S-12 进程管理：服务端过滤排序（命令行已脱敏）+ 结束进程（带 `started_at` 防 pid 复用）。
 * 受保护进程（init / 内核线程 / 面板及其祖先）按钮禁用，后端同样拒绝。 */
const { t } = useI18n()
const toast = useFpToast()

const searchText = ref('')
const search = refDebounced(searchText, 300)
const userFilter = ref<string | undefined>(undefined)
const sort = ref<ProcessSortField>('cpu')
const order = ref<'asc' | 'desc'>('desc')
const limit = ref(100)
const includeKernel = ref(false)
const autoRefresh = ref(true)
const first = ref(0)

const params = computed(() => ({
  q: search.value,
  user: userFilter.value || undefined,
  sort: sort.value,
  order: order.value,
  limit: limit.value,
  include_kernel: includeKernel.value,
}))

// 条件变化回到第一页
watch(params, () => (first.value = 0))

const { data, rows, loading, error, refresh, kill, killing } = useProcesses(params, {
  autoRefresh,
})

/** 已有数据时后台刷新失败不打断页面 */
const loadError = computed(() => (data.value ? null : error.value))
const truncated = computed(() => !!data.value && data.value.matched > rows.value.length)

// 运行时长每 30 s 走一次即可（自动刷新关闭时也保持大致准确）
const now = ref(Date.now())
useIntervalFn(() => {
  now.value = Date.now()
}, 30_000)
watch(data, () => (now.value = Date.now()))

const summary = computed<LayoutSummaryItem[]>(() => {
  if (!data.value) return []
  return [
    { label: t('processes.summary.total'), value: data.value.total, tone: 'brand' },
    { label: t('processes.summary.matched'), value: data.value.matched, tone: 'info' },
    {
      label: t('processes.summary.memory'),
      value: formatSize(data.value.total_memory_bytes),
      tone: 'muted',
    },
  ]
})

const userSelectOptions = computed(() =>
  userOptions(rows.value, userFilter.value).map((u) => ({ label: u, value: u })),
)
const sortOptions = computed(() =>
  PROCESS_SORTS.map((value) => ({ value, label: t(`processes.sort.${value}`) })),
)
const orderOptions = computed(() =>
  (['desc', 'asc'] as const).map((value) => ({ value, label: t(`processes.order.${value}`) })),
)
const limitOptions = [50, 100, 200, 500].map((n) => ({
  value: n,
  label: t('processes.limit', { n }),
}))

const killOpen = ref(false)
const killTarget = ref<ProcessRow | null>(null)

function openKill(row: ProcessRow) {
  if (row.protected) return
  killTarget.value = row
  killOpen.value = true
}

async function onConfirmKill(signal: ProcessSignal) {
  const target = killTarget.value
  if (!target) return
  try {
    await kill(target.pid, { signal, started_at: target.started_at })
    toast.success(
      t('processes.kill.success', { name: target.name, pid: target.pid, signal: `SIG${signal}` }),
    )
    killOpen.value = false
  } catch (e) {
    const key = killErrorKey(e)
    if (!key) {
      toast.error(e, t('processes.kill.failed'))
      return
    }
    toast.warning(t(key, { pid: target.pid }))
    // 进程已退出 / pid 被复用：关窗并刷新列表；受保护 / 权限不足：保留弹窗便于用户改选
    if (key === 'processes.kill.gone' || key === 'processes.kill.reused') {
      killOpen.value = false
      void refresh()
    }
  }
}
</script>

<style scoped>
.toolbar-left,
.toolbar-right {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
}
.toolbar-right {
  margin-left: auto;
}
.toolbar-search {
  width: 240px;
}
.toolbar-filter {
  width: 150px;
}
.toolbar-order {
  width: 110px;
}
.truncated-hint {
  margin: 0 0 var(--fp-space-2);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
}
@media (max-width: 960px) {
  .toolbar-right {
    margin-left: 0;
  }
  .toolbar-search {
    width: 100%;
  }
}
</style>
