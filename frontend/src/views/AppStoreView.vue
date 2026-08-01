<template>
  <div class="app-store">
    <div class="header">
      <h2>{{ t('appStore.title') }}</h2>
      <div class="header-actions">
        <el-input
          v-model="search"
          :placeholder="t('appStore.searchPlaceholder')"
          clearable
          class="search-input"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-button @click="showImportDialog = true">
          <el-icon><FolderOpened /></el-icon>
          <span>{{ t('appStore.import') }}</span>
        </el-button>
      </div>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane :label="t('appStore.tabStore')" name="store">
        <el-row v-loading="loading" :gutter="16">
          <el-col v-for="pkg in filteredPackages" :key="pkg.key" :span="6" class="app-card-col">
            <el-card shadow="hover" class="app-card">
              <div class="app-card-body">
                <div class="app-logo" :class="pkg.category === 'wasm' ? 'wasm-logo' : ''">
                  <el-icon><Box /></el-icon>
                </div>
                <div class="app-info">
                  <div class="app-name">{{ pkg.name }}</div>
                  <div class="app-desc">{{ pkg.short_desc_zh }}</div>
                  <div class="app-tags">
                    <el-tag size="small" type="info">{{ formatLabel(pkg.format) }}</el-tag>
                    <el-tag v-for="tag in pkg.tags" :key="tag" size="small" class="tag">{{ tag }}</el-tag>
                  </div>
                </div>
                <div class="app-actions">
                  <el-button size="small" type="primary" @click="openInstall(pkg)">
                    {{ t('appStore.install') }}
                  </el-button>
                </div>
              </div>
            </el-card>
          </el-col>
        </el-row>
        <el-empty v-if="!loading && filteredPackages.length === 0" :description="t('appStore.empty')" />
      </el-tab-pane>

      <el-tab-pane :label="t('appStore.tabInstalled')" name="installed">
        <el-table v-loading="loadingInstalled" :data="installedApps" style="width: 100%">
          <el-table-column prop="name" :label="t('appStore.colName')" min-width="140" />
          <el-table-column prop="package_key" :label="t('appStore.colPackage')" min-width="120" />
          <el-table-column prop="version" :label="t('appStore.colVersion')" width="100" />
          <el-table-column :label="t('appStore.colMode')" width="100">
            <template #default="{ row }">
              <el-tag size="small" :type="modeTagType(row.mode)">{{ modeLabel(row.mode) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('appStore.colStatus')" width="100">
            <template #default="{ row }">
              <el-tag size="small" :type="row.status === 'running' ? 'success' : 'danger'">
                {{ row.status }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('appStore.colAccess')" min-width="160">
            <template #default="{ row }">
              <el-link v-if="row.access_url" type="primary" :href="row.access_url" target="_blank">
                {{ row.access_url }}
              </el-link>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('appStore.colActions')" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="showLogs(row)">{{ t('appStore.logs') }}</el-button>
              <el-button size="small" @click="upgrade(row)">{{ t('appStore.upgrade') }}</el-button>
              <el-button size="small" type="danger" @click="uninstall(row)">
                {{ t('appStore.uninstall') }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane :label="t('appStore.tabWasm')" name="wasm">
        <el-row v-loading="loadingWasm" :gutter="16">
          <el-col v-for="pkg in wasmBuiltins" :key="pkg.key" :span="6" class="app-card-col">
            <el-card shadow="hover" class="app-card">
              <div class="app-card-body">
                <div class="app-logo wasm-logo">
                  <el-icon><MagicStick /></el-icon>
                </div>
                <div class="app-info">
                  <div class="app-name">{{ pkg.name }}</div>
                  <div class="app-desc">{{ pkg.short_desc_zh }}</div>
                </div>
                <div class="app-actions">
                  <el-button size="small" type="primary" @click="openInstall(pkg)">
                    {{ t('appStore.install') }}
                  </el-button>
                </div>
              </div>
            </el-card>
          </el-col>
        </el-row>
        <el-empty v-if="!loadingWasm && wasmBuiltins.length === 0" :description="t('appStore.empty')" />
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="installVisible" :title="t('appStore.installTitle') + ' - ' + (installMeta?.name || '')" width="640px">
      <el-alert
        v-if="securityWarning"
        type="warning"
        :title="t('appStore.securityWarning')"
        :description="securityWarning"
        show-icon
        :closable="false"
        class="security-alert"
      />
      <el-form v-loading="installLoading" :model="installForm" label-width="120px">
        <el-form-item :label="t('appStore.fldVersion')">
          <el-select v-model="installForm.version">
            <el-option v-for="v in installMeta?.versions || []" :key="v" :label="v" :value="v" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('appStore.fldMode')">
          <el-select v-model="installForm.mode">
            <el-option
              v-for="m in installMeta?.modes || []"
              :key="m"
              :label="modeLabel(m)"
              :value="m"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('appStore.fldName')">
          <el-input v-model="installForm.name" :placeholder="t('appStore.fldNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('appStore.fldPort')">
          <el-input-number v-model="installForm.port" :min="1" :max="65535" />
        </el-form-item>
        <el-form-item :label="t('appStore.fldContainer')">
          <el-input v-model="installForm.container_name" />
        </el-form-item>
        <template v-if="versionInfo">
          <el-divider>{{ t('appStore.parameters') }}</el-divider>
          <el-form-item
            v-for="f in versionInfo.form_fields"
            :key="f.env_key"
            :label="f.label_zh"
            :required="f.required"
          >
            <el-select v-if="f.field_type === 'select'" v-model="installForm.values[f.env_key]">
              <el-option v-for="o in f.options" :key="o.value" :label="o.label" :value="o.value" />
            </el-select>
            <el-switch v-else-if="f.field_type === 'switch'" v-model="switchValues[f.env_key]" />
            <el-input-number
              v-else-if="f.field_type === 'number' || f.field_type === 'port'"
              v-model="numberValues[f.env_key]"
              :min="f.min ?? undefined"
              :max="f.max ?? undefined"
            />
            <el-input
              v-else
              v-model="installForm.values[f.env_key]"
              :type="f.field_type === 'password' ? 'password' : 'text'"
              :placeholder="f.description || ''"
            />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-checkbox v-model="installForm.confirm_risky">
          {{ t('appStore.confirmRisky') }}
        </el-checkbox>
        <el-button @click="installVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="installLoading" @click="doInstall">
          {{ t('appStore.install') }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="logsVisible" :title="t('appStore.logs')" width="700px">
      <pre class="logs-view">{{ currentLogs }}</pre>
      <template #footer>
        <el-button type="primary" @click="logsVisible = false">{{ t('common.close') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showImportDialog" :title="t('appStore.import')" width="480px">
      <el-form label-width="100px">
        <el-form-item :label="t('appStore.importPath')">
          <el-input v-model="importPath" placeholder="/opt/flamepanel/apps/myapp" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showImportDialog = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="doImport">{{ t('appStore.import') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, FolderOpened, Box, MagicStick } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import {
  listPackages,
  getPackage,
  getPackageVersion,
  installApp,
  importPackage,
  listInstalledApps,
  uninstallApp,
  upgradeApp,
  getAppLogs,
  listWasmBuiltins,
  type AppMetadata,
  type AppVersionInfo,
  type InstalledApp,
} from '@/api/appStore'

const { t } = useI18n()
const activeTab = ref('store')
const search = ref('')
const loading = ref(false)
const loadingInstalled = ref(false)
const loadingWasm = ref(false)
const packages = ref<AppMetadata[]>([])
const installedApps = ref<InstalledApp[]>([])
const wasmBuiltins = ref<AppMetadata[]>([])
const installVisible = ref(false)
const installLoading = ref(false)
const installMeta = ref<AppMetadata | null>(null)
const versionInfo = ref<AppVersionInfo | null>(null)
const securityWarning = ref('')
const logsVisible = ref(false)
const currentLogs = ref('')
const showImportDialog = ref(false)
const importPath = ref('')

const switchValues = reactive<Record<string, boolean>>({})
const numberValues = reactive<Record<string, number>>({})

const installForm = reactive<{
  version: string
  mode: string
  name: string
  port?: number
  container_name: string
  confirm_risky: boolean
  values: Record<string, string>
}>({
  version: '',
  mode: '',
  name: '',
  container_name: '',
  confirm_risky: false,
  values: {},
})

const filteredPackages = computed(() => {
  if (!search.value) return packages.value
  const q = search.value.toLowerCase()
  return packages.value.filter(
    (p) => p.name.toLowerCase().includes(q) || p.key.toLowerCase().includes(q) || p.short_desc_zh.toLowerCase().includes(q),
  )
})


function errorDetail(e: unknown): string {
  const err = e as { response?: { data?: { detail?: string } } }
  return err.response?.data?.detail ?? ''
}

function formatLabel(format: string): string {
  const map: Record<string, string> = {
    flame: 'Flame',
    onepanel: '1Panel',
    baota: '宝塔',
  }
  return map[format] || format
}

function modeLabel(mode: string): string {
  const map: Record<string, string> = {
    container: t('appStore.modeContainer'),
    native: t('appStore.modeNative'),
    wasm: 'WASM',
  }
  return map[mode] || mode
}

function modeTagType(mode: string): 'info' | 'success' | 'warning' {
  if (mode === 'wasm') return 'warning'
  if (mode === 'native') return 'success'
  return 'info'
}

async function loadPackages() {
  loading.value = true
  try {
    const res = await listPackages()
    packages.value = res.data.packages
  } finally {
    loading.value = false
  }
}

async function loadInstalled() {
  loadingInstalled.value = true
  try {
    const res = await listInstalledApps()
    installedApps.value = res.data
  } finally {
    loadingInstalled.value = false
  }
}

async function loadWasm() {
  loadingWasm.value = true
  try {
    const res = await listWasmBuiltins()
    wasmBuiltins.value = res.data
  } finally {
    loadingWasm.value = false
  }
}

async function openInstall(pkg: AppMetadata) {
  installMeta.value = pkg
  installForm.version = pkg.default_version
  installForm.mode = pkg.modes[0] ?? 'container'
  installForm.name = pkg.name
  installForm.port = undefined
  installForm.container_name = ''
  installForm.confirm_risky = false
  installForm.values = {}
  securityWarning.value = ''
  installVisible.value = true
  try {
    const meta = await getPackage(pkg.key)
    installMeta.value = meta.data
  } catch {
    // builtin wasm tools have no package record
  }
  await loadVersion()
}

async function loadVersion() {
  versionInfo.value = null
  if (!installMeta.value) return
  try {
    const res = await getPackageVersion(installMeta.value.key, installForm.version)
    versionInfo.value = res.data
    for (const f of res.data.form_fields ?? []) {
      if (f.default && !(f.env_key in installForm.values)) {
        installForm.values[f.env_key] = f.default
      }
      if (f.field_type === 'switch') switchValues[f.env_key] = f.default === 'true'
      if (f.field_type === 'number' || f.field_type === 'port') {
        numberValues[f.env_key] = f.default ? Number(f.default) : 0
      }
    }
    const compose = res.data.compose_template ?? ''
    if (compose.includes('privileged: true') && !installForm.confirm_risky) {
      securityWarning.value = t('appStore.warnPrivileged')
    } else if (compose.includes('/var/run/docker.sock') && !installForm.confirm_risky) {
      securityWarning.value = t('appStore.warnDockerSock')
    }
  } catch {
    // version info unavailable
  }
}

watch(() => installForm.version, loadVersion)

async function doInstall() {
  if (!installMeta.value) return
  installLoading.value = true
  try {
    const values: Record<string, string> = { ...installForm.values }
    for (const key of Object.keys(switchValues)) values[key] = String(switchValues[key])
    for (const key of Object.keys(numberValues)) {
      if (numberValues[key] !== 0) values[key] = String(numberValues[key])
    }
    await installApp(installMeta.value.key, {
      package_key: installMeta.value.key,
      version: installForm.version,
      mode: installForm.mode,
      name: installForm.name,
      port: installForm.port,
      container_name: installForm.container_name,
      values,
      confirm_risky: installForm.confirm_risky,
    })
    ElMessage.success(t('appStore.installSuccess'))
    installVisible.value = false
    loadInstalled()
  } catch (e: unknown) {
    ElMessage.error(errorDetail(e) || t('appStore.installFailed'))
  } finally {
    installLoading.value = false
  }
}

async function uninstall(row: InstalledApp) {
  try {
    await ElMessageBox.confirm(t('appStore.confirmUninstall', { name: row.name }), t('common.warning'), {
      type: 'warning',
    })
  } catch {
    return
  }
  try {
    await uninstallApp(row.id)
    ElMessage.success(t('appStore.uninstallSuccess'))
    loadInstalled()
  } catch (e: unknown) {
    ElMessage.error(errorDetail(e) || t('appStore.uninstallFailed'))
  }
}

async function upgrade(row: InstalledApp) {
  try {
    await upgradeApp(row.id)
    ElMessage.success(t('appStore.upgradeSuccess'))
    loadInstalled()
  } catch (e: unknown) {
    ElMessage.error(errorDetail(e) || t('appStore.upgradeFailed'))
  }
}

async function showLogs(row: InstalledApp) {
  try {
    const res = await getAppLogs(row.id)
    currentLogs.value = res.data.logs || '-'
    logsVisible.value = true
  } catch {
    currentLogs.value = '-'
    logsVisible.value = true
  }
}

async function doImport() {
  if (!importPath.value) return
  try {
    await importPackage(importPath.value)
    ElMessage.success(t('appStore.importSuccess'))
    showImportDialog.value = false
    loadPackages()
  } catch (e: unknown) {
    ElMessage.error(errorDetail(e) || t('appStore.importFailed'))
  }
}

onMounted(() => {
  loadPackages()
  loadInstalled()
  loadWasm()
})
</script>

<style scoped>
.app-store {
  padding: 20px;
}
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.search-input {
  width: 260px;
}
.app-card-col {
  margin-bottom: 16px;
}
.app-card-body {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}
.app-logo {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  background: var(--el-color-primary-light-9);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  color: var(--el-color-primary);
  flex-shrink: 0;
}
.wasm-logo {
  background: #f0f5ff;
  color: #4f6ef7;
}
.app-info {
  flex: 1;
  min-width: 0;
}
.app-name {
  font-weight: 600;
  margin-bottom: 4px;
}
.app-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 8px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.app-tags .tag {
  margin-left: 4px;
}
.app-actions {
  flex-shrink: 0;
}
.security-alert {
  margin-bottom: 12px;
}
.logs-view {
  background: #0f172a;
  color: #a5f3fc;
  padding: 12px;
  border-radius: 6px;
  max-height: 420px;
  overflow-y: auto;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
