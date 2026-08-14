<template>
  <LayoutContent :title="t('database.title')" reload @reload="invalidate">
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
          <FpButton variant="primary" @click="showInstallMysql = true">{{
            t('database.installMysql')
          }}</FpButton>
          <FpButton variant="warning" @click="showInstallRedis = true">{{
            t('database.installRedis')
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
              <FpButton v-permission="{ perm: 'database:start', mode: 'view' }" variant="primary" :disabled="data.status === 'running'" @click="handleStart(data.id)">
                {{ t('database.start') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'database:stop', mode: 'view' }" variant="warning" :disabled="data.status !== 'running'" @click="handleStop(data.id)">
                {{ t('database.stop') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'database:start', mode: 'view' }" variant="ghost" @click="handleRestart(data.id)">
                {{ t('database.restart') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'database:delete', mode: 'view' }" variant="danger" @click="handleUninstall(data.id)">
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

    <FpModal v-model="showInstallMysql" :header="t('database.installMysql')">
      <div class="modal-form">
        <FpInput v-model="mysqlForm.name" :label="t('database.name')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="mysqlForm.version" :label="t('database.version')" :placeholder="t('database.versionPlaceholder')" />
        <div class="field-col">
          <label class="field-label">{{ t('database.port') }}</label>
          <FpNumber v-model="mysqlForm.port" :min="1024" :max="65535" class="w-full" />
        </div>
        <FpInput v-model="mysqlForm.root_password" :label="t('database.rootPassword')" type="password" toggle-mask />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showInstallMysql = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="installing" @click="handleInstallMysql">
          {{ t('common.install') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showInstallRedis" :header="t('database.installRedis')">
      <div class="modal-form">
        <FpInput v-model="redisForm.name" :label="t('database.name')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="redisForm.version" :label="t('database.version')" :placeholder="t('database.versionPlaceholder')" />
        <div class="field-col">
          <label class="field-label">{{ t('database.port') }}</label>
          <FpNumber v-model="redisForm.port" :min="1024" :max="65535" class="w-full" />
        </div>
        <FpInput
          v-model="redisForm.password"
          :label="t('database.password')"
          type="password"
          toggle-mask
          :placeholder="t('database.optional')"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showInstallRedis = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="installing" @click="handleInstallRedis">
          {{ t('common.install') }}
        </FpButton>
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'



import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import {
  listDatabases,
  installMysql,
  installRedis,
  startDatabase,
  stopDatabase,
  restartDatabase,
  uninstallDatabase,
} from '@/api/databases'
import type { DatabaseInstance } from '@/types'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { useCrudPage } from '@/composables/useCrudPage'

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
const typeOptions = computed(() => [
  { label: 'MySQL', value: 'mysql' },
  { label: 'Redis', value: 'redis' },
])
const filteredInstances = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return instances.value.filter((d) => {
    if (typeFilter.value && d.db_type !== typeFilter.value) return false
    if (!kw) return true
    return (
      d.name.toLowerCase().includes(kw) ||
      d.db_type.toLowerCase().includes(kw) ||
      String(d.port).includes(kw)
    )
  })
})
const installing = ref(false)
const showInstallMysql = ref(false)
const showInstallRedis = ref(false)

function initForm(port = 3306, name = '') {
  return { name, version: '', port, root_password: '', password: '' }
}
const mysqlForm = ref(initForm(3306, 'MySQL 8'))
const redisForm = ref(initForm(6379, 'Redis 7'))

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

async function handleInstallMysql() {
  installing.value = true
  try {
    await installMysql(mysqlForm.value)
    toast.success(t('common.success'))
    showInstallMysql.value = false
    invalidate()
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    installing.value = false
  }
}

async function handleInstallRedis() {
  installing.value = true
  try {
    await installRedis(redisForm.value)
    toast.success(t('common.success'))
    showInstallRedis.value = false
    invalidate()
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    installing.value = false
  }
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
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.field-col {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
</style>
