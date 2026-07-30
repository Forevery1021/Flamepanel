<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.databases') }}</h2>
      <div>
        <el-button type="primary" @click="showInstallMysql = true">{{ t('database.installMysql') }}</el-button>
        <el-button type="warning" @click="showInstallRedis = true">{{ t('database.installRedis') }}</el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table :data="instances" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="name" :label="t('database.name')" />
        <el-table-column prop="db_type" :label="t('database.type')" width="100" />
        <el-table-column prop="version" :label="t('database.version')" width="140" />
        <el-table-column prop="port" :label="t('database.port')" width="80" />
        <el-table-column :label="t('database.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'running' ? 'success' : 'danger'" size="small">{{ row.status === 'running' ? t('database.running') : t('database.stopped') }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="data_dir" :label="t('database.dataDir')" />
        <el-table-column :label="t('database.actions')" width="280">
          <template #default="{ row }">
            <el-button size="small" :disabled="row.status === 'running'" @click="handleStart(row.id)">{{ t('database.start') }}</el-button>
            <el-button size="small" :disabled="row.status !== 'running'" @click="handleStop(row.id)">{{ t('database.stop') }}</el-button>
            <el-button size="small" @click="handleRestart(row.id)">{{ t('database.restart') }}</el-button>
            <el-button size="small" type="danger" @click="handleUninstall(row.id)">{{ t('database.uninstall') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        background
        small
        style="margin-top: 16px; justify-content: center;"
        @current-change="fetch"
      />
    </el-card>

    <el-dialog v-model="showInstallMysql" :title="t('database.installMysql')" width="500">
      <el-form :model="mysqlForm" :label="t('database.name')">
        <el-form-item :label="t('database.name')">
          <el-input v-model="mysqlForm.name" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-form-item :label="t('database.version')">
          <el-input v-model="mysqlForm.version" :placeholder="t('database.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('database.port')">
          <el-input-number v-model="mysqlForm.port" :min="1024" :max="65535" />
        </el-form-item>
        <el-form-item :label="t('database.rootPassword')">
          <el-input v-model="mysqlForm.root_password" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstallMysql = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="handleInstallMysql" :loading="installing">{{ t('common.install') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showInstallRedis" :title="t('database.installRedis')" width="500">
      <el-form :model="redisForm" label-width="140">
        <el-form-item :label="t('database.name')">
          <el-input v-model="redisForm.name" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-form-item :label="t('database.version')">
          <el-input v-model="redisForm.version" :placeholder="t('database.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('database.port')">
          <el-input-number v-model="redisForm.port" :min="1024" :max="65535" />
        </el-form-item>
        <el-form-item :label="t('database.password')">
          <el-input v-model="redisForm.password" type="password" show-password :placeholder="t('database.optional')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstallRedis = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="handleInstallRedis" :loading="installing">{{ t('common.install') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listDatabases, installMysql, installRedis, startDatabase, stopDatabase, restartDatabase, uninstallDatabase } from '@/api/databases'
import type { DatabaseInstance } from '@/types'

const { t } = useI18n()
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
  }
  finally { loading.value = false }
}

async function handleInstallMysql() {
  installing.value = true
  try {
    await installMysql(mysqlForm.value)
    ElMessage.success(t('common.success'))
    showInstallMysql.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    installing.value = false
  }
}

async function handleInstallRedis() {
  installing.value = true
  try {
    await installRedis(redisForm.value)
    ElMessage.success(t('common.success'))
    showInstallRedis.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    installing.value = false
  }
}

async function handleStart(id: number) {
  try {
    await startDatabase(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleStop(id: number) {
  try {
    await stopDatabase(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleRestart(id: number) {
  try {
    await restartDatabase(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleUninstall(id: number) {
  try {
    await ElMessageBox.confirm(t('common.confirmAction'), t('common.confirm'))
    await uninstallDatabase(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch { /* cancelled or failed */ }
}

onMounted(fetch)
</script>

<style scoped>
h2 { margin: 0; }
</style>
