<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.webServers') }}</h2>
      <div>
        <el-button type="primary" @click="showInstall = true">{{ t('webServer.install') }}</el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table :data="instances" v-loading="loading" stripe style="width:100%">
        <el-table-column prop="id" :label="t('webServer.id')" width="60" />
        <el-table-column prop="engine" :label="t('webServer.engine')" width="120" />
        <el-table-column prop="version" :label="t('webServer.version')" width="100" />
        <el-table-column prop="port" :label="t('webServer.port')" width="80" />
        <el-table-column :label="t('webServer.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'running' ? 'success' : 'info'" size="small">
              {{ row.status === 'running' ? t('webServer.running') : t('webServer.stopped') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="config_path" :label="t('webServer.configPath')" min-width="160" show-overflow-tooltip />
        <el-table-column :label="t('webServer.actions')" width="420" fixed="right">
          <template #default="{ row }">
            <el-button size="small" :disabled="row.status === 'running'" @click="handleStart(row.id)">{{ t('webServer.start') }}</el-button>
            <el-button size="small" :disabled="row.status !== 'running'" @click="handleStop(row.id)">{{ t('webServer.stop') }}</el-button>
            <el-button size="small" @click="handleRestart(row.id)">{{ t('webServer.restart') }}</el-button>
            <el-button size="small" @click="handleReload(row.id)">{{ t('webServer.reload') }}</el-button>
            <el-button size="small" @click="handleConfigtest(row.id)">{{ t('webServer.configtest') }}</el-button>
            <el-popconfirm :title="t('webServer.deleteConfirm', { name: row.engine + '-' + row.id })" @confirm="handleDelete(row.id)">
              <template #reference>
                <el-button size="small" type="danger">{{ t('webServer.uninstall') }}</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="showInstall" :title="t('webServer.installTitle')" width="500px">
      <el-form :model="form" label-width="120px">
        <el-form-item :label="t('webServer.engine')" required>
          <el-select v-model="form.engine" style="width:100%">
            <el-option v-for="e in engines" :key="e.name" :label="e.name" :value="e.name" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('webServer.version')">
          <el-input v-model="form.version" :placeholder="t('webServer.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('webServer.configPath')">
          <el-input v-model="form.config_path" :placeholder="t('webServer.configPathPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('webServer.binaryPath')">
          <el-input v-model="form.binary_path" :placeholder="t('webServer.binaryPathPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('webServer.port')">
          <el-input-number v-model="form.port" :min="1" :max="65535" style="width:100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstall = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="handleInstall" :loading="submitting">{{ t('common.install') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { listEngines, listWebServers, createWebServer, deleteWebServer, startWebServer, stopWebServer, restartWebServer, reloadWebServer, configtestWebServer } from '@/api/webServers'
import type { EngineInfo, WebServerResponse } from '@/types'

const { t } = useI18n()
const instances = ref<WebServerResponse[]>([])
const engines = ref<EngineInfo[]>([])
const loading = ref(false)
const submitting = ref(false)
const showInstall = ref(false)
const form = ref({ engine: '', version: '', config_path: '', binary_path: '', port: 80 })

async function fetch() {
  loading.value = true
  try {
    const [inst, eng] = await Promise.all([
      listWebServers(),
      listEngines().catch(() => ({ data: [] as EngineInfo[] })),
    ])
    instances.value = inst.data
    engines.value = eng.data
  } catch { ElMessage.error(t('common.failed')) }
  finally { loading.value = false }
}

async function handleInstall() {
  if (!form.value.engine) { ElMessage.warning(t('common.required')); return }
  submitting.value = true
  try {
    const data: Record<string, any> = { engine: form.value.engine }
    if (form.value.version) data.version = form.value.version
    if (form.value.config_path) data.config_path = form.value.config_path
    if (form.value.binary_path) data.binary_path = form.value.binary_path
    if (form.value.port) data.port = form.value.port
    await createWebServer(data as any)
    ElMessage.success(t('common.success'))
    showInstall.value = false
    form.value = { engine: '', version: '', config_path: '', binary_path: '', port: 80 }
    await fetch()
  } catch { ElMessage.error(t('common.failed')) }
  finally { submitting.value = false }
}

async function handleStart(id: number) { try { await startWebServer(id); ElMessage.success(t('common.success')); await fetch() } catch { ElMessage.error(t('common.failed')) } }
async function handleStop(id: number) { try { await stopWebServer(id); ElMessage.success(t('common.success')); await fetch() } catch { ElMessage.error(t('common.failed')) } }
async function handleRestart(id: number) { try { await restartWebServer(id); ElMessage.success(t('common.success')); await fetch() } catch { ElMessage.error(t('common.failed')) } }
async function handleReload(id: number) { try { await reloadWebServer(id); ElMessage.success(t('common.success')); await fetch() } catch { ElMessage.error(t('common.failed')) } }
async function handleConfigtest(id: number) { try { await configtestWebServer(id); ElMessage.success(t('common.success')) } catch { ElMessage.error(t('common.failed')) } }
async function handleDelete(id: number) { try { await deleteWebServer(id); ElMessage.success(t('common.success')); await fetch() } catch { ElMessage.error(t('common.failed')) } }

onMounted(fetch)
</script>
