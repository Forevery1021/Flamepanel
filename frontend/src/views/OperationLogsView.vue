<template>
  <LayoutContent :title="t('log.title')" reload @reload="fetch">
    <template #toolbar>
      <FpSelect
        v-model="actionFilter"
        :options="actionOptions"
        option-label="label"
        option-value="value"
        show-clear
        class="filter-select"
        @update:model-value="fetch"
      />
    </template>

    <div class="panel">
      <FpTable
        :rows="logs"
        :loading="loading"
        :first="(currentPage - 1) * pageSize"
        :empty-text="t('common.noData')"
        striped-rows
        virtual
        virtual-scroll-height="620px"
      >
        <FpColumn field="id" :header="t('log.id')" style="width: 60px" />
        <FpColumn field="username" :header="t('log.user')" style="width: 120px" />
        <FpColumn :header="t('log.action')" style="width: 180px">
          <template #body="{ data }">
            <FpTag :severity="actionSeverity(data.action)" :value="data.action" />
          </template>
        </FpColumn>
        <FpColumn field="target" :header="t('log.target')" style="min-width: 200px" />
        <FpColumn field="ip" :header="t('log.ip')" style="width: 140px" />
        <FpColumn field="created_at" :header="t('log.time')" style="width: 180px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </FpColumn>
      </FpTable>
      <FpPagination
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => goPage(f)"
      />
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'


import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import { listOperationLogs } from '@/api/logs'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type { OperationLog } from '@/types'

const { t } = useI18n()
const currentPage = ref(1)
const pageSize = ref(20)
const actionFilter = ref('')

const actionOptions = computed(() => [
  { label: '全部', value: '' },
  { label: t('log.loginActions'), value: 'LOGIN' },
  { label: 'POST', value: 'POST' },
  { label: 'PUT', value: 'PUT' },
  { label: 'DELETE', value: 'DELETE' },
])

function actionSeverity(action: string): 'success' | 'warning' | 'danger' | 'info' | 'neutral' {
  if (action.startsWith('LOGIN')) return 'warning'
  if (action.startsWith('DELETE')) return 'danger'
  return 'info'
}

// F1.1：分页走统一数据获取层，切换页/筛选保留上一页数据（keepPreviousData）
const logsQuery = useApiQuery<{ data: OperationLog[]; total: number }>(
  () => queryKeys.operationLogs.list(currentPage.value, pageSize.value, actionFilter.value),
  async () => {
    const res = await listOperationLogs(
      currentPage.value,
      pageSize.value,
      actionFilter.value || undefined,
    )
    return { data: { data: res.data.data, total: res.data.total } }
  },
  { keepPrevious: true },
)
const logs = computed<OperationLog[]>(() => logsQuery.data.value?.data ?? [])
const loading = logsQuery.loading
const total = computed(() => logsQuery.data.value?.total ?? 0)

async function fetch() {
  await logsQuery.refresh()
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  void fetch()
}
</script>

<style scoped>
.filter-select {
  width: 160px;
}
</style>
