<template>
  <div class="view-container">
    <el-tabs v-model="activeTab">
      <el-tab-pane :label="t('webServer.instances')" name="instances">
    <div class="card-header-title">
      <el-button type="primary" @click="showInstall = true">{{
        t('webServer.install')
      }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :empty-text="t('common.noData')"
        :data="instances"
        stripe
        class="full-width"
      >
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
        <el-table-column
          prop="config_path"
          :label="t('webServer.configPath')"
          min-width="160"
          show-overflow-tooltip
        />
        <el-table-column :label="t('webServer.actions')" width="420" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              :disabled="row.status === 'running'"
              @click="handleStart(row.id)"
              >{{ t('webServer.start') }}</el-button
            >
            <el-button
              size="small"
              :disabled="row.status !== 'running'"
              @click="handleStop(row.id)"
              >{{ t('webServer.stop') }}</el-button
            >
            <el-button size="small" @click="handleRestart(row.id)">{{
              t('webServer.restart')
            }}</el-button>
            <el-button size="small" @click="handleReload(row.id)">{{
              t('webServer.reload')
            }}</el-button>
            <el-button size="small" @click="handleConfigtest(row.id)">{{
              t('webServer.configtest')
            }}</el-button>
            <el-button size="small" @click="openPreset(row)">{{ t('webServer.preset') }}</el-button>
            <el-button size="small" @click="openSwitch(row)">{{ t('webServer.switchEngine') }}</el-button>
            <el-popconfirm
              :title="t('webServer.deleteConfirm', { name: row.engine + '-' + row.id })"
              @confirm="handleDelete(row.id)"
            >
              <template #reference>
                <el-button size="small" type="danger">{{ t('webServer.uninstall') }}</el-button>
              </template>
            </el-popconfirm>
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
        class="table-pagination"
        @current-change="fetch"
      />
    </el-card>
      </el-tab-pane>

      <el-tab-pane :label="t('webServer.nativeTab')" name="native">
        <el-card shadow="hover">
          <div class="native-toolbar">
            <el-button size="small" type="primary" :loading="detecting" @click="fetchNative">{{
              t('webServer.nativeDetect')
            }}</el-button>
            <el-alert
              :title="t('webServer.nativeHint')"
              type="info"
              :closable="false"
              class="native-hint"
            />
          </div>
          <el-table
            v-loading="detecting"
            :empty-text="t('common.noData')"
            :data="nativeList"
            stripe
            class="full-width mt-2"
          >
            <el-table-column prop="engine" :label="t('webServer.engine')" width="130" />
            <el-table-column :label="t('webServer.status')" width="110">
              <template #default="{ row }">
                <el-tag :type="row.installed ? 'success' : 'info'" size="small">
                  {{
                    row.installed
                      ? t('webServer.nativeInstalled')
                      : t('webServer.nativeNotInstalled')
                  }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="version" :label="t('webServer.nativeVersion')" width="100">
              <template #default="{ row }">{{ row.version || '—' }}</template>
            </el-table-column>
            <el-table-column :label="t('webServer.nativeRunning')" width="90">
              <template #default="{ row }">
                <el-tag :type="row.running ? 'success' : 'info'" size="small" effect="plain">
                  {{ row.running ? t('webServer.nativeRunning') : t('webServer.nativeStopped') }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="t('webServer.nativeAutostart')" width="110">
              <template #default="{ row }">
                <el-switch
                  :model-value="row.enabled"
                  :disabled="!row.installed"
                  @change="(val: string | number | boolean) => handleNativeAutostart(row, Boolean(val))"
                />
              </template>
            </el-table-column>
            <el-table-column :label="t('webServer.nativeListenPorts')" width="140">
              <template #default="{ row }">
                <el-tag
                  v-for="p in row.listening_ports"
                  :key="p"
                  size="small"
                  class="mr-1"
                  effect="plain"
                  >{{ p }}</el-tag
                >
                <span v-if="!row.listening_ports || !row.listening_ports.length">—</span>
              </template>
            </el-table-column>
            <el-table-column
              prop="binary_path"
              :label="t('webServer.nativePath')"
              min-width="160"
              show-overflow-tooltip
            >
              <template #default="{ row }">{{ row.binary_path || '—' }}</template>
            </el-table-column>
            <el-table-column :label="t('webServer.actions')" width="180" fixed="right">
              <template #default="{ row }">
                <el-popconfirm
                  v-if="!row.installed"
                  :title="t('webServer.nativeInstallConfirm', { name: row.engine })"
                  @confirm="handleNativeInstall(row.engine)"
                >
                  <template #reference>
                    <el-button size="small" type="primary">{{
                      t('webServer.nativeInstallBtn')
                    }}</el-button>
                  </template>
                </el-popconfirm>
                <el-popconfirm
                  v-else
                  :title="t('webServer.nativeUninstallConfirm', { name: row.engine })"
                  @confirm="handleNativeUninstall(row.engine)"
                >
                  <template #reference>
                    <el-button size="small" type="danger">{{
                      t('webServer.nativeUninstallBtn')
                    }}</el-button>
                  </template>
                </el-popconfirm>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>


    <el-dialog v-model="showInstall" :title="t('webServer.installTitle')" width="500px">
      <el-form :model="form" label-width="120px">
        <el-form-item :label="t('webServer.engine')" required>
          <el-select v-model="form.engine" class="full-width">
            <el-option v-for="e in engines" :key="e.name" :label="e.name" :value="e.name" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('webServer.version')">
          <el-input v-model="form.version" :placeholder="t('webServer.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('webServer.configPath')">
          <el-input
            v-model="form.config_path"
            :placeholder="t('webServer.configPathPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('webServer.binaryPath')">
          <el-input
            v-model="form.binary_path"
            :placeholder="t('webServer.binaryPathPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('webServer.port')">
          <el-input-number v-model="form.port" :min="1" :max="65535" class="full-width" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showInstall = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleInstall">{{
          t('common.install')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showPreset" :title="t('webServer.preset')" width="520px">
      <el-alert
        :title="t('webServer.presetRecommend')"
        type="info"
        :closable="false"
        class="preset-alert"
      />
      <el-radio-group v-model="selectedPreset" class="preset-group">
        <el-radio v-for="p in presets" :key="p.name" :label="p.name" class="preset-radio">
          {{ p.description }} <el-tag v-if="p.recommended" size="small" type="success">推荐</el-tag>
        </el-radio>
      </el-radio-group>
      <template #footer>
        <el-button @click="showPreset = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleApplyPreset">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showSwitch" :title="t('webServer.switchEngine')" width="480px">
      <el-form label-width="120px">
        <el-form-item :label="t('webServer.engine')" required>
          <el-select v-model="switchEngine" class="full-width">
            <el-option v-for="e in engines" :key="e.name" :label="e.name" :value="e.name" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <span class="switch-tip">{{ t('webServer.switchTip') }}</span>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showSwitch = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSwitchEngine">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import {
  listEngines,
  listWebServers,
  createWebServer,
  deleteWebServer,
  startWebServer,
  stopWebServer,
  restartWebServer,
  reloadWebServer,
  configtestWebServer,
  switchWebServerEngine,
  applyWebServerPreset,
  listPresets,
  detectNativeWebServers,
  nativeInstallWebServer,
  nativeUninstallWebServer,
  nativeAutostartWebServer,
} from '@/api/webServers'
import type {
  EngineInfo,
  WebServerResponse,
  PerformancePresetInfo,
  NativeWebServerInfo,
} from '@/types'

const { t } = useI18n()
const activeTab = ref('instances')
const instances = ref<WebServerResponse[]>([])
const engines = ref<EngineInfo[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const submitting = ref(false)
const showInstall = ref(false)
const form = ref({ engine: '', version: '', config_path: '', binary_path: '', port: 80 })

async function fetch() {
  loading.value = true
  try {
    const [inst, eng] = await Promise.all([
      listWebServers(currentPage.value, pageSize.value),
      listEngines().catch(() => ({ data: [] as EngineInfo[] })),
    ])
    instances.value = inst.data.data
    total.value = inst.data.total
    engines.value = eng.data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function handleInstall() {
  if (!form.value.engine) {
    ElMessage.warning(t('common.required'))
    return
  }
  submitting.value = true
  try {
    const data: {
      engine: string
      version?: string
      config_path?: string
      binary_path?: string
      port?: number
    } = { engine: form.value.engine }
    if (form.value.version) data.version = form.value.version
    if (form.value.config_path) data.config_path = form.value.config_path
    if (form.value.binary_path) data.binary_path = form.value.binary_path
    if (form.value.port) data.port = form.value.port
    await createWebServer(data)
    ElMessage.success(t('common.success'))
    showInstall.value = false
    form.value = { engine: '', version: '', config_path: '', binary_path: '', port: 80 }
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function handleStart(id: number) {
  try {
    await startWebServer(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function handleStop(id: number) {
  try {
    await stopWebServer(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function handleRestart(id: number) {
  try {
    await restartWebServer(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function handleReload(id: number) {
  try {
    await reloadWebServer(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function handleConfigtest(id: number) {
  try {
    await configtestWebServer(id)
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function handleDelete(id: number) {
  try {
    await deleteWebServer(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

const showPreset = ref(false)
const showSwitch = ref(false)
const presets = ref<PerformancePresetInfo[]>([])
const selectedPreset = ref('')
const switchEngine = ref('')
let targetServerId = 0

async function openPreset(row: WebServerResponse) {
  targetServerId = row.id
  try {
    const res = await listPresets()
    presets.value = res.data
    const recommended = presets.value.find((p) => p.recommended)
    selectedPreset.value = recommended ? recommended.name : 'medium'
    showPreset.value = true
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleApplyPreset() {
  if (!selectedPreset.value) return
  submitting.value = true
  try {
    await applyWebServerPreset(targetServerId, selectedPreset.value)
    ElMessage.success(t('common.success'))
    showPreset.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function openSwitch(row: WebServerResponse) {
  targetServerId = row.id
  switchEngine.value = row.engine
  showSwitch.value = true
}

async function handleSwitchEngine() {
  if (!switchEngine.value) return
  submitting.value = true
  try {
    await switchWebServerEngine(targetServerId, switchEngine.value)
    ElMessage.success(t('common.success'))
    showSwitch.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

// ── 原生控制 ──
const nativeList = ref<NativeWebServerInfo[]>([])
const detecting = ref(false)
const nativeBusy = ref(false)

async function fetchNative() {
  detecting.value = true
  try {
    const res = await detectNativeWebServers()
    nativeList.value = res.data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    detecting.value = false
  }
}

async function handleNativeInstall(engine: string) {
  if (nativeBusy.value) return
  nativeBusy.value = true
  try {
    await nativeInstallWebServer(engine)
    ElMessage.success(t('common.success'))
    await fetchNative()
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    nativeBusy.value = false
  }
}

async function handleNativeUninstall(engine: string) {
  if (nativeBusy.value) return
  nativeBusy.value = true
  try {
    await nativeUninstallWebServer(engine)
    ElMessage.success(t('common.success'))
    await fetchNative()
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    nativeBusy.value = false
  }
}

async function handleNativeAutostart(row: NativeWebServerInfo, enabled: boolean) {
  try {
    await nativeAutostartWebServer(row.engine, enabled)
    ElMessage.success(t('common.success'))
    await fetchNative()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

onMounted(() => {
  fetch()
  fetchNative()
})
</script>

<style scoped>
.native-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.native-hint {
  flex: 1;
  min-width: 260px;
}
.mr-1 {
  margin-right: 4px;
}
.mt-2 {
  margin-top: 8px;
}
</style>
