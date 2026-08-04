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
        scrollable
        scroll-height="620px"
      >
        <Column field="id" :header="t('log.id')" style="width: 60px" />
        <Column field="username" :header="t('log.user')" style="width: 120px" />
        <Column :header="t('log.action')" style="width: 180px">
          <template #body="{ data }">
            <FpTag :severity="actionSeverity(data.action)" :value="data.action" />
          </template>
        </Column>
        <Column field="target" :header="t('log.target')" style="min-width: 200px" />
        <Column field="ip" :header="t('log.ip')" style="width: 140px" />
        <Column field="created_at" :header="t('log.time')" style="width: 180px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </Column>
      </FpTable>
      <Paginator
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total-records="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => goPage(f)"
      />
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { listOperationLogs } from '@/api/logs'
import type { OperationLog } from '@/types'

const { t } = useI18n()
const logs = ref<OperationLog[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
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

async function fetch() {
  loading.value = true
  try {
    const res = await listOperationLogs(currentPage.value, pageSize.value, actionFilter.value || undefined)
    logs.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
}

onMounted(fetch)
</script>

<style scoped>
.filter-select {
  width: 160px;
}
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
</style>
