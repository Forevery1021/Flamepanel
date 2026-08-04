<template>
  <LayoutContent :title="t('database.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton variant="primary" @click="showInstallMysql = true">{{
        t('database.installMysql')
      }}</FpButton>
      <FpButton variant="warning" @click="showInstallRedis = true">{{
        t('database.installRedis')
      }}</FpButton>
    </template>

    <div class="panel">
      <FpTable
        :rows="instances"
        :loading="loading"
        :first="(currentPage - 1) * pageSize"
        :empty-text="t('common.noData')"
        striped-rows
      >
        <Column field="name" :header="t('database.name')" />
        <Column field="db_type" :header="t('database.type')" style="width: 100px" />
        <Column field="version" :header="t('database.version')" style="width: 140px" />
        <Column field="port" :header="t('database.port')" style="width: 80px" />
        <Column :header="t('database.status')" style="width: 100px">
          <template #body="{ data }">
            <FpTag
              :severity="data.status === 'running' ? 'success' : 'danger'"
              :value="data.status === 'running' ? t('database.running') : t('database.stopped')"
              dot
            />
          </template>
        </Column>
        <Column field="data_dir" :header="t('database.dataDir')" />
        <Column :header="t('database.actions')" style="width: 280px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="primary" :disabled="data.status === 'running'" @click="handleStart(data.id)">
                {{ t('database.start') }}
              </FpButton>
              <FpButton variant="warning" :disabled="data.status !== 'running'" @click="handleStop(data.id)">
                {{ t('database.stop') }}
              </FpButton>
              <FpButton variant="ghost" @click="handleRestart(data.id)">
                {{ t('database.restart') }}
              </FpButton>
              <FpButton variant="danger" @click="handleUninstall(data.id)">
                {{ t('database.uninstall') }}
              </FpButton>
            </div>
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

    <FpModal v-model="showInstallMysql" :header="t('database.installMysql')">
      <div class="modal-form">
        <FpInput v-model="mysqlForm.name" :label="t('database.name')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="mysqlForm.version" :label="t('database.version')" :placeholder="t('database.versionPlaceholder')" />
        <div class="field-col">
          <label class="field-label">{{ t('database.port') }}</label>
          <InputNumber v-model="mysqlForm.port" :min="1024" :max="65535" class="w-full" />
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
          <InputNumber v-model="redisForm.port" :min="1024" :max="65535" class="w-full" />
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
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
import InputNumber from 'openvue/inputnumber'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
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

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const instances = ref<DatabaseInstance[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const installing = ref(false)
const showInstallMysql = ref(false)
const showInstallRedis = ref(false)

function initForm(port = 3306, name = '') {
  return { name, version: '', port, root_password: '', password: '' }
}
const mysqlForm = ref(initForm(3306, 'MySQL 8'))
const redisForm = ref(initForm(6379, 'Redis 7'))

async function fetch() {
  loading.value = true
  try {
    const res = await listDatabases(currentPage.value, pageSize.value)
    instances.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
}

async function handleInstallMysql() {
  installing.value = true
  try {
    await installMysql(mysqlForm.value)
    toast.success(t('common.success'))
    showInstallMysql.value = false
    await fetch()
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
    await fetch()
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
    await fetch()
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

async function handleStop(id: number) {
  try {
    await stopDatabase(id)
    toast.success(t('common.success'))
    await fetch()
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

async function handleRestart(id: number) {
  try {
    await restartDatabase(id)
    toast.success(t('common.success'))
    await fetch()
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
        await fetch()
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

onMounted(fetch)
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
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
