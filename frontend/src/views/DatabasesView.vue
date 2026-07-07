<template>
  <div class="databases-view">
    <div class="header">
      <h2>Databases</h2>
      <el-button type="primary" @click="showInstallMysql = true">Install MySQL</el-button>
      <el-button type="warning" @click="showInstallRedis = true">Install Redis</el-button>
    </div>

    <el-table :data="instances" v-loading="loading" stripe style="width: 100%; margin-top: 16px">
      <el-table-column prop="name" label="Name" />
      <el-table-column prop="db_type" label="Type" width="100" />
      <el-table-column prop="version" label="Version" width="140" />
      <el-table-column prop="port" label="Port" width="80" />
      <el-table-column label="Status" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === 'running' ? 'success' : 'info'">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="data_dir" label="Data Dir" />
      <el-table-column label="Actions" width="300" fixed="right">
        <template #default="{ row }">
          <el-button size="small" type="success" @click="handleStart(row)" :disabled="row.status === 'running'">Start</el-button>
          <el-button size="small" type="warning" @click="handleStop(row)" :disabled="row.status !== 'running'">Stop</el-button>
          <el-button size="small" @click="handleRestart(row)">Restart</el-button>
          <el-button size="small" type="danger" @click="handleUninstall(row)">Uninstall</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="showInstallMysql" title="Install MySQL" width="500">
      <el-form :model="mysqlForm" label-width="140">
        <el-form-item label="Instance Name">
          <el-input v-model="mysqlForm.name" placeholder="e.g. MySQL 8.0" />
        </el-form-item>
        <el-form-item label="Version">
          <el-input v-model="mysqlForm.version" placeholder="latest (optional)" />
        </el-form-item>
        <el-form-item label="Port">
          <el-input-number v-model="mysqlForm.port" :min="1024" :max="65535" />
        </el-form-item>
        <el-form-item label="Root Password">
          <el-input v-model="mysqlForm.root_password" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstallMysql = false">Cancel</el-button>
        <el-button type="primary" @click="handleInstallMysql" :loading="installing">Install</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showInstallRedis" title="Install Redis" width="500">
      <el-form :model="redisForm" label-width="140">
        <el-form-item label="Instance Name">
          <el-input v-model="redisForm.name" placeholder="e.g. Redis 7" />
        </el-form-item>
        <el-form-item label="Version">
          <el-input v-model="redisForm.version" placeholder="latest (optional)" />
        </el-form-item>
        <el-form-item label="Port">
          <el-input-number v-model="redisForm.port" :min="1024" :max="65535" />
        </el-form-item>
        <el-form-item label="Password">
          <el-input v-model="redisForm.password" type="password" show-password placeholder="optional" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstallRedis = false">Cancel</el-button>
        <el-button type="primary" @click="handleInstallRedis" :loading="installing">Install</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listDatabases, installMysql, installRedis, startDatabase, stopDatabase, restartDatabase, uninstallDatabase } from '@/api/databases'
import type { DatabaseInstance } from '@/types'

const instances = ref<DatabaseInstance[]>([])
const loading = ref(false)
const installing = ref(false)
const showInstallMysql = ref(false)
const showInstallRedis = ref(false)

const mysqlForm = ref({ name: 'MySQL', version: '', port: 3306, root_password: '' })
const redisForm = ref({ name: 'Redis', version: '', port: 6379, password: '' })

async function fetch() {
  loading.value = true
  try {
    const res = await listDatabases()
    instances.value = res.data
  } catch {
    ElMessage.error('Failed to fetch databases')
  } finally {
    loading.value = false
  }
}

async function handleInstallMysql() {
  installing.value = true
  try {
    await installMysql(mysqlForm.value)
    ElMessage.success('MySQL installed')
    showInstallMysql.value = false
    await fetch()
  } catch {
    ElMessage.error('Installation failed')
  } finally {
    installing.value = false
  }
}

async function handleInstallRedis() {
  installing.value = true
  try {
    await installRedis(redisForm.value)
    ElMessage.success('Redis installed')
    showInstallRedis.value = false
    await fetch()
  } catch {
    ElMessage.error('Installation failed')
  } finally {
    installing.value = false
  }
}

async function handleStart(row: DatabaseInstance) {
  try {
    await startDatabase(row.id)
    ElMessage.success(`${row.name} started`)
    await fetch()
  } catch {
    ElMessage.error('Start failed')
  }
}

async function handleStop(row: DatabaseInstance) {
  try {
    await stopDatabase(row.id)
    ElMessage.success(`${row.name} stopped`)
    await fetch()
  } catch {
    ElMessage.error('Stop failed')
  }
}

async function handleRestart(row: DatabaseInstance) {
  try {
    await restartDatabase(row.id)
    ElMessage.success(`${row.name} restarted`)
    await fetch()
  } catch {
    ElMessage.error('Restart failed')
  }
}

async function handleUninstall(row: DatabaseInstance) {
  try {
    await ElMessageBox.confirm(`Uninstall ${row.name}? This will remove the database software.`, 'Confirm')
    await uninstallDatabase(row.id)
    ElMessage.success(`${row.name} uninstalled`)
    await fetch()
  } catch {
    /* cancelled or failed */
  }
}

onMounted(fetch)
</script>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: center; }
h2 { margin: 0; }
</style>
