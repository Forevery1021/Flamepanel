<template>
  <LayoutContent
    :title="t('database.title')"
    :description="t('pageDesc.databases')"
    reload
    @reload="invalidate"
  >
    <template #toolbar>
      <!-- P4：统一 PageToolbar -->
      <PageToolbar v-model="searchText">
        <template #left>
          <FpSelect
            v-model="typeFilter"
            :options="typeOptions"
            option-label="label"
            option-value="value"
            show-clear
            class="toolbar-filter"
          />
        </template>
        <template #actions>
          <FpButton variant="primary" @click="installEngine = 'mysql'">{{
            t('database.installMysql')
          }}</FpButton>
          <FpButton variant="warning" @click="installEngine = 'redis'">{{
            t('database.installRedis')
          }}</FpButton>
          <FpButton variant="secondary" @click="installEngine = 'postgres'">{{
            t('database.installPostgres')
          }}</FpButton>
          <FpButton variant="secondary" @click="installEngine = 'mongodb'">{{
            t('database.installMongodb')
          }}</FpButton>
        </template>
      </PageToolbar>
    </template>

    <div class="panel">
      <FpTable
        :rows="filteredInstances"
        :loading="loading"
        :first="first"
        :empty-text="t('common.noData')"
        :empty-desc="t('pageDesc.emptyHint')"
        striped-rows
      >
        <FpColumn field="name" :header="t('database.name')" />
        <FpColumn field="db_type" :header="t('database.type')" style="width: 100px" />
        <FpColumn field="version" :header="t('database.version')" style="width: 140px" />
        <FpColumn field="port" :header="t('database.port')" style="width: 80px" />
        <FpColumn :header="t('database.status')" style="width: 100px">
          <template #body="{ data }">
            <FpTag
              :severity="data.status === 'running' ? 'success' : 'danger'"
              :value="data.status === 'running' ? t('database.running') : t('database.stopped')"
              dot
            />
          </template>
        </FpColumn>
        <FpColumn field="data_dir" :header="t('database.dataDir')" />
        <FpColumn :header="t('database.actions')" style="width: 280px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton
                v-permission="{ perm: 'database:start', mode: 'view' }"
                variant="primary"
                :disabled="data.status === 'running'"
                @click="handleStart(data.id)"
              >
                {{ t('database.start') }}
              </FpButton>
              <FpButton
                v-permission="{ perm: 'database:stop', mode: 'view' }"
                variant="warning"
                :disabled="data.status !== 'running'"
                @click="handleStop(data.id)"
              >
                {{ t('database.stop') }}
              </FpButton>
              <FpButton
                v-permission="{ perm: 'database:start', mode: 'view' }"
                variant="ghost"
                @click="handleRestart(data.id)"
              >
                {{ t('database.restart') }}
              </FpButton>
              <FpButton
                v-permission="{ perm: 'database:delete', mode: 'view' }"
                variant="danger"
                @click="handleUninstall(data.id)"
              >
                {{ t('database.uninstall') }}
              </FpButton>
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
        @update:first="(f) => onFirst(f)"
      />
    </div>

    <DatabaseInstallModal v-model:engine="installEngine" @done="invalidate" />
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import {
  listDatabases,
  startDatabase,
  stopDatabase,
  restartDatabase,
  uninstallDatabase,
} from '@/api/databases'
import type { DatabaseInstance } from '@/types'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { useCrudPage } from '@/composables/useCrudPage'
import DatabaseInstallModal from '@/components/databases/DatabaseInstallModal.vue'
import { engineTypeOptions, filterInstances, type DatabaseEngine } from '@/utils/databases'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()
// P4：统一 CRUD 分页状态
const crud = useCrudPage()
const { total, first, pageSize, onFirst } = crud

// M9：搜索 + 类型筛选（当前页客户端过滤）
const searchText = ref('')
const typeFilter = ref<string>('')
const typeOptions = engineTypeOptions()
const filteredInstances = computed(() =>
  filterInstances(instances.value, searchText.value, typeFilter.value),
)
const installEngine = ref<DatabaseEngine | null>(null)

// P3-A：列表走统一数据获取层 useApiQuery
const databasesQuery = useApiQuery<{ data: DatabaseInstance[]; total: number }>(
  () => queryKeys.databases.list(crud.currentPage.value, crud.pageSize.value),
  async () => {
    const res = await listDatabases(crud.currentPage.value, crud.pageSize.value)
    crud.total.value = res.data.total
    return { data: { data: res.data.data, total: res.data.total } }
  },
  { keepPrevious: true },
)
const instances = computed<DatabaseInstance[]>(() => databasesQuery.data.value?.data ?? [])
const loading = databasesQuery.loading

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.databases.all })
}

async function handleStart(id: number) {
  try {
    await startDatabase(id)
    toast.success(t('common.success'))
    invalidate()
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

async function handleStop(id: number) {
  try {
    await stopDatabase(id)
    toast.success(t('common.success'))
    invalidate()
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

async function handleRestart(id: number) {
  try {
    await restartDatabase(id)
    toast.success(t('common.success'))
    invalidate()
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

function handleUninstall(id: number) {
  confirmAction({
    message: t('common.confirmAction'),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await uninstallDatabase(id)
        toast.success(t('common.success'))
        invalidate()
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}
</script>

<style scoped>
.toolbar-filter {
  width: 160px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
