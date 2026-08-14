<template>
  <div class="view-container">
    <div class="page-toolbar">
      <div class="search-box">
        <i class="oi oi-search search-icon" />
        <FpInput v-model="search" :placeholder="t('appStore.searchPlaceholder')" />
      </div>
      <FpButton v-permission="{ perm: 'app_store:create', mode: 'view' }" variant="ghost" icon="oi oi-folder-open" @click="showImportDialog = true">
        {{ t('appStore.import') }}
      </FpButton>
    </div>

    <FpTabs v-model="activeTab" class="store-tabs" :items="tabItems">
<template #store>
<!-- 推荐安装 -->
          <div v-if="recommendedPackages.length" class="recommend-section">
            <div class="section-title">{{ t('appStore.recommended') }}</div>
            <div class="recommend-list">
              <div
                v-for="pkg in recommendedPackages"
                :key="pkg.key"
                class="recommend-card"
                @click="openInstall(pkg)"
              >
                <div class="recommend-logo"><i class="oi oi-box" /></div>
                <div class="recommend-info">
                  <div class="recommend-name">{{ pkg.name }}</div>
                  <div class="recommend-desc">{{ pkg.short_desc_zh }}</div>
                  <FpTag severity="warning" :value="t('appStore.recommended')" />
                </div>
                <FpButton v-permission="{ perm: 'app_store:create', mode: 'view' }" variant="primary" @click.stop="openInstall(pkg)">
                  {{ installedKeys.has(pkg.key) ? t('appStore.installed') : t('appStore.install') }}
                </FpButton>
              </div>
            </div>
          </div>

          <div v-if="loading" class="app-grid">
            <div v-for="i in 8" :key="i" class="app-card app-card-skeleton">
              <FpSkeleton height="44px" />
              <FpSkeleton height="16px" />
              <FpSkeleton height="12px" />
            </div>
          </div>
          <div v-else-if="filteredPackages.length" class="app-grid">
            <div v-for="pkg in filteredPackages" :key="pkg.key" class="app-card">
              <div class="app-card-body">
                <div class="app-logo" :class="{ 'wasm-logo': pkg.category === 'wasm' }">
                  <i class="oi oi-box" />
                </div>
                <div class="app-info">
                  <div class="app-name">{{ pkg.name }}</div>
                  <div class="app-desc">{{ pkg.short_desc_zh }}</div>
                  <div class="app-tags">
                    <FpTag severity="info" :value="formatLabel(pkg.format)" />
                    <FpTag v-for="tag in pkg.tags" :key="tag" :value="tag" />
                  </div>
                </div>
              </div>
              <div class="app-actions">
                <FpButton
                  :variant="installedKeys.has(pkg.key) ? 'success' : 'primary'"
                  @click="openInstall(pkg)"
                >
                  {{ installedKeys.has(pkg.key) ? t('appStore.installed') : t('appStore.install') }}
                </FpButton>
              </div>
            </div>
          </div>
          <FpEmpty v-else :description="t('appStore.empty')" icon="oi oi-inbox" />
</template>
<template #installed>
<div class="panel">
            <FpTable
              :rows="installedApps"
              :loading="loadingInstalled"
              :empty-text="t('appStore.empty')"
              virtual
              virtual-scroll-height="560px"
            >
              <FpColumn field="name" :header="t('appStore.colName')" style="min-width: 140px" />
              <FpColumn field="package_key" :header="t('appStore.colPackage')" style="min-width: 120px" />
              <FpColumn field="version" :header="t('appStore.colVersion')" style="width: 100px" />
              <FpColumn :header="t('appStore.colMode')" style="width: 100px">
                <template #body="{ data }">
                  <FpTag :severity="modeTagType(data.mode)" :value="modeLabel(data.mode)" />
                </template>
              </FpColumn>
              <FpColumn :header="t('appStore.colStatus')" style="width: 100px">
                <template #body="{ data }">
                  <FpTag
                    :severity="data.status === 'running' ? 'success' : 'danger'"
                    :value="data.status"
                  />
                </template>
              </FpColumn>
              <FpColumn :header="t('appStore.colAccess')" style="min-width: 160px">
                <template #body="{ data }">
                  <a
                    v-if="data.access_url"
                    class="access-link"
                    :href="data.access_url"
                    target="_blank"
                  >{{ data.access_url }}</a>
                  <span v-else>-</span>
                </template>
              </FpColumn>
              <FpColumn :header="t('appStore.colActions')" style="width: 240px" frozen>
                <template #body="{ data }">
                  <div class="row-actions">
                    <FpButton variant="ghost" @click="showLogs(data)">{{ t('appStore.logs') }}</FpButton>
                    <FpButton v-permission="{ perm: 'app_store:update', mode: 'view' }" variant="ghost" @click="upgrade(data)">{{ t('appStore.upgrade') }}</FpButton>
                    <FpButton v-permission="{ perm: 'app_store:delete', mode: 'view' }" variant="danger" @click="uninstall(data)">
                      {{ t('appStore.uninstall') }}
                    </FpButton>
                  </div>
                </template>
              </FpColumn>
            </FpTable>
          </div>
</template>
<template #wasm>
<div v-if="loadingWasm" class="app-grid">
            <div v-for="i in 8" :key="i" class="app-card app-card-skeleton">
              <FpSkeleton height="44px" />
              <FpSkeleton height="16px" />
              <FpSkeleton height="12px" />
            </div>
          </div>
          <div v-else-if="wasmBuiltins.length" class="app-grid">
            <div v-for="pkg in wasmBuiltins" :key="pkg.key" class="app-card">
              <div class="app-card-body">
                <div class="app-logo wasm-logo"><i class="oi oi-sparkles" /></div>
                <div class="app-info">
                  <div class="app-name">{{ pkg.name }}</div>
                  <div class="app-desc">{{ pkg.short_desc_zh }}</div>
                </div>
              </div>
              <div class="app-actions">
                <FpButton variant="primary" @click="openInstall(pkg)">
                  {{ t('appStore.install') }}
                </FpButton>
              </div>
            </div>
          </div>
          <FpEmpty v-else :description="t('appStore.empty')" icon="oi oi-inbox" />
</template>
</FpTabs>

    <FpModal
      v-model="installVisible"
      :header="t('appStore.installTitle') + ' - ' + (installMeta?.name || '')"
      style="width: 640px"
    >
      <div v-if="securityWarning" class="security-alert">
        <i class="oi oi-exclamation-triangle" />
        <div>
          <div class="security-title">{{ t('appStore.securityWarning') }}</div>
          <div>{{ securityWarning }}</div>
        </div>
      </div>
      <div class="modal-form">
        <FpSelect
          v-model="installForm.version"
          :label="t('appStore.fldVersion')"
          :options="versionOptions"
          option-label="label"
          option-value="value"
        />
        <FpSelect
          v-model="installForm.mode"
          :label="t('appStore.fldMode')"
          :options="modeOptions"
          option-label="label"
          option-value="value"
        />
        <FpInput
          v-model="installForm.name"
          :label="t('appStore.fldName')"
          :placeholder="t('appStore.fldNamePlaceholder')"
        />
        <div class="field-col">
          <label class="field-label">{{ t('appStore.fldPort') }}</label>
          <FpNumber v-model="installForm.port" :min="1" :max="65535" class="w-full" />
        </div>
        <FpInput v-model="installForm.container_name" :label="t('appStore.fldContainer')" />
        <template v-if="versionInfo">
          <FpDivider>{{ t('appStore.parameters') }}</FpDivider>
          <div v-for="f in versionInfo.form_fields" :key="f.env_key" class="modal-form">
            <FpSelect
              v-if="f.field_type === 'select'"
              v-model="installForm.values[f.env_key]"
              :label="f.label_zh"
              :options="f.options"
              option-label="label"
              option-value="value"
            />
            <div v-else-if="f.field_type === 'switch'" class="field-col field-row">
              <label class="field-label">{{ f.label_zh }}</label>
              <FpSwitch v-model="switchValues[f.env_key]" />
            </div>
            <div
              v-else-if="f.field_type === 'number' || f.field_type === 'port'"
              class="field-col"
            >
              <label class="field-label">{{ f.label_zh }}</label>
              <FpNumber
                v-model="numberValues[f.env_key]"
                :min="f.min ?? undefined"
                :max="f.max ?? undefined"
                class="w-full"
              />
            </div>
            <FpInput
              v-else
              v-model="installForm.values[f.env_key]"
              :label="f.label_zh"
              :type="f.field_type === 'password' ? 'password' : 'text'"
              :placeholder="f.description || ''"
            />
          </div>
        </template>
      </div>
      <template #footer>
        <div class="install-footer">
          <FpCheckbox v-model="installForm.confirm_risky">
            {{ t('appStore.confirmRisky') }}
          </FpCheckbox>
          <div class="footer-btns">
            <FpButton variant="ghost" @click="installVisible = false">
              {{ t('common.cancel') }}
            </FpButton>
            <FpButton variant="primary" :loading="installLoading" @click="doInstall">
              {{ t('appStore.install') }}
            </FpButton>
          </div>
        </div>
      </template>
    </FpModal>

    <FpModal v-model="logsVisible" :header="t('appStore.logs')" style="width: 700px">
      <pre class="logs-view">{{ currentLogs }}</pre>
      <template #footer>
        <FpButton variant="primary" @click="logsVisible = false">{{ t('common.close') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showImportDialog" :header="t('appStore.import')" style="width: 480px">
      <FpInput
        v-model="importPath"
        :label="t('appStore.importPath')"
        placeholder="/opt/flamepanel/apps/myapp"
      />
      <template #footer>
        <FpButton variant="ghost" @click="showImportDialog = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" @click="doImport">{{ t('appStore.import') }}</FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'











import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpEmpty from '@/components/ui/FpEmpty.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
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

const tabItems: FpTabItem[] = [
  { value: 'store', label: t('appStore.tabStore') },
  { value: 'installed', label: t('appStore.tabInstalled') },
  { value: 'wasm', label: t('appStore.tabWasm') },
]
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

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

// 推荐安装（后端 recommended 标记）
const recommendedPackages = computed(() =>
  packages.value.filter((p) => p.recommended),
)
// 已安装应用 key 集合（角标/按钮状态）
const installedKeys = computed(() => new Set(installedApps.value.map((a) => a.package_key)))

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

const versionOptions = computed(() =>
  (installMeta.value?.versions || []).map((v) => ({ label: v, value: v })),
)
const modeOptions = computed(() =>
  (installMeta.value?.modes || []).map((m) => ({ label: modeLabel(m), value: m })),
)

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
    toast.success(t('appStore.installSuccess'))
    installVisible.value = false
    loadInstalled()
  } catch (e: unknown) {
    toast.error(e, errorDetail(e) || t('appStore.installFailed'))
  } finally {
    installLoading.value = false
  }
}

function uninstall(row: InstalledApp) {
  confirmAction({
    message: t('appStore.confirmUninstall', { name: row.name }),
    header: t('common.warning'),
    accept: async () => {
      try {
        await uninstallApp(row.id)
        toast.success(t('appStore.uninstallSuccess'))
        loadInstalled()
      } catch (e: unknown) {
        toast.error(e, errorDetail(e) || t('appStore.uninstallFailed'))
      }
    },
  })
}

async function upgrade(row: InstalledApp) {
  try {
    await upgradeApp(row.id)
    toast.success(t('appStore.upgradeSuccess'))
    loadInstalled()
  } catch (e: unknown) {
    toast.error(e, errorDetail(e) || t('appStore.upgradeFailed'))
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
    toast.success(t('appStore.importSuccess'))
    showImportDialog.value = false
    loadPackages()
  } catch (e: unknown) {
    toast.error(e, errorDetail(e) || t('appStore.importFailed'))
  }
}

onMounted(() => {
  loadPackages()
  loadInstalled()
  loadWasm()
})
</script>

<style scoped>
.view-container {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.page-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--fp-space-3);
}
.search-box {
  position: relative;
  width: 260px;
}
.search-icon {
  position: absolute;
  left: var(--fp-space-3);
  top: 50%;
  transform: translateY(-50%);
  z-index: 1;
  color: var(--fp-text-muted);
  font-size: 14px;
  pointer-events: none;
}
.search-box :deep(input) {
  padding-left: 34px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: var(--fp-space-3);
}
.recommend-section {
  margin-bottom: var(--fp-space-4);
}
.recommend-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--fp-space-3);
}
.recommend-card {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-lg);
  border: 1px solid var(--fp-border);
  background: var(--fp-bg-elevated);
  cursor: pointer;
  transition:
    border-color var(--fp-transition-fast),
    box-shadow var(--fp-transition-fast);
}
.recommend-card:hover {
  border-color: var(--fp-brand);
  box-shadow: var(--fp-shadow-lg);
}
.recommend-logo {
  width: 44px;
  height: 44px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}
.recommend-info {
  flex: 1;
  min-width: 0;
}
.recommend-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.recommend-desc {
  font-size: 12px;
  color: var(--fp-text-secondary);
  margin: 2px 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.app-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--fp-space-3);
}
.app-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  transition:
    box-shadow var(--fp-transition-fast),
    transform 120ms var(--fp-ease-out);
}
.app-card:hover {
  box-shadow: var(--fp-shadow-lg);
  transform: translateY(-1px);
}
.app-card-skeleton {
  gap: var(--fp-space-3);
}
.app-card-body {
  display: flex;
  gap: var(--fp-space-3);
  align-items: flex-start;
}
.app-logo {
  width: 44px;
  height: 44px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}
.wasm-logo {
  background: var(--fp-info-soft);
  color: var(--fp-info);
}
.app-info {
  flex: 1;
  min-width: 0;
}
.app-name {
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: 4px;
}
.app-desc {
  font-size: 12px;
  color: var(--fp-text-secondary);
  margin-bottom: var(--fp-space-2);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.app-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-1);
}
.app-actions {
  display: flex;
  justify-content: flex-end;
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.field-col {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.field-row {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}
.field-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.security-alert {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-md);
  background: var(--fp-warning-soft);
  border: 1px solid var(--fp-warning);
  font-size: 13px;
  color: var(--fp-text-primary);
  margin-bottom: var(--fp-space-4);
}
.security-alert i {
  color: var(--fp-warning);
  margin-top: 2px;
}
.security-title {
  font-weight: 600;
  color: var(--fp-warning);
  margin-bottom: 2px;
}
.logs-view {
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  max-height: 420px;
  overflow-y: auto;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: var(--fp-font-mono);
}
.install-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  gap: var(--fp-space-3);
}
.footer-btns {
  display: flex;
  gap: var(--fp-space-2);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.access-link {
  color: var(--fp-brand);
  text-decoration: none;
}
.access-link:hover {
  text-decoration: underline;
}
</style>
