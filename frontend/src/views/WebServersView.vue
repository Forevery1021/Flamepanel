<template>
  <div class="view-container">
    <Tabs v-model:value="activeTab" class="servers-tabs">
      <TabList>
        <Tab value="instances">{{ t('webServer.instances') }}</Tab>
        <Tab value="native">{{ t('webServer.nativeTab') }}</Tab>
      </TabList>
      <TabPanels>
        <TabPanel value="instances">
          <div class="page-toolbar">
            <FpButton variant="primary" icon="oi oi-plus" @click="showInstall = true">
              {{ t('webServer.install') }}
            </FpButton>
          </div>

          <div class="panel">
            <FpTable
              :rows="instances"
              :loading="loading"
              :empty-text="t('common.noData')"
              :first="(currentPage - 1) * pageSize"
              striped-rows
            >
              <Column field="id" :header="t('webServer.id')" style="width: 60px" />
              <Column field="engine" :header="t('webServer.engine')" style="width: 120px" />
              <Column :header="t('webServer.version')" style="width: 100px">
                <template #body="{ data }">{{ data.version || '—' }}</template>
              </Column>
              <Column field="port" :header="t('webServer.port')" style="width: 80px" />
              <Column :header="t('webServer.status')" style="width: 110px">
                <template #body="{ data }">
                  <FpTag
                    :severity="data.status === 'running' ? 'success' : 'info'"
                    :dot="data.status === 'running'"
                  >
                    {{ data.status === 'running' ? t('webServer.running') : t('webServer.stopped') }}
                  </FpTag>
                </template>
              </Column>
              <Column :header="t('webServer.configPath')" style="min-width: 160px">
                <template #body="{ data }">
                  <span v-tooltip="data.config_path" class="cell-truncate">{{ data.config_path }}</span>
                </template>
              </Column>
              <Column :header="t('webServer.actions')" style="width: 460px" frozen>
                <template #body="{ data }">
                  <div class="row-actions">
                    <FpButton
                      variant="ghost"
                      :disabled="data.status === 'running'"
                      @click="handleStart(data.id)"
                      >{{ t('webServer.start') }}</FpButton
                    >
                    <FpButton
                      variant="ghost"
                      :disabled="data.status !== 'running'"
                      @click="handleStop(data.id)"
                      >{{ t('webServer.stop') }}</FpButton
                    >
                    <FpButton variant="ghost" @click="handleRestart(data.id)">{{
                      t('webServer.restart')
                    }}</FpButton>
                    <FpButton variant="ghost" @click="handleReload(data.id)">{{
                      t('webServer.reload')
                    }}</FpButton>
                    <FpButton variant="ghost" @click="handleConfigtest(data.id)">{{
                      t('webServer.configtest')
                    }}</FpButton>
                    <FpButton variant="ghost" @click="openPreset(data)">{{
                      t('webServer.preset')
                    }}</FpButton>
                    <FpButton variant="ghost" @click="openSwitch(data)">{{
                      t('webServer.switchEngine')
                    }}</FpButton>
                    <FpButton variant="danger" @click="confirmDelete(data)">{{
                      t('webServer.uninstall')
                    }}</FpButton>
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
        </TabPanel>

        <TabPanel value="native">
          <div class="panel">
            <div class="native-toolbar">
              <FpButton variant="primary" icon="oi oi-refresh" :loading="detecting" @click="fetchNative">
                {{ t('webServer.nativeDetect') }}
              </FpButton>
              <InlineMessage severity="info" class="native-hint">
                {{ t('webServer.nativeHint') }}
              </InlineMessage>
            </div>
            <FpTable
              :rows="nativeList"
              :loading="detecting"
              :empty-text="t('common.noData')"
              striped-rows
              class="mt-2"
            >
              <Column field="engine" :header="t('webServer.engine')" style="width: 130px" />
              <Column :header="t('webServer.status')" style="width: 110px">
                <template #body="{ data }">
                  <FpTag :severity="data.installed ? 'success' : 'info'">
                    {{
                      data.installed
                        ? t('webServer.nativeInstalled')
                        : t('webServer.nativeNotInstalled')
                    }}
                  </FpTag>
                </template>
              </Column>
              <Column :header="t('webServer.nativeVersion')" style="width: 100px">
                <template #body="{ data }">{{ data.version || '—' }}</template>
              </Column>
              <Column :header="t('webServer.nativeRunning')" style="width: 90px">
                <template #body="{ data }">
                  <FpTag :severity="data.running ? 'success' : 'info'">
                    {{ data.running ? t('webServer.nativeRunning') : t('webServer.nativeStopped') }}
                  </FpTag>
                </template>
              </Column>
              <Column :header="t('webServer.nativeAutostart')" style="width: 110px">
                <template #body="{ data }">
                  <ToggleSwitch
                    :model-value="data.enabled"
                    :disabled="!data.installed"
                    @update:model-value="(v) => handleNativeAutostart(data, Boolean(v))"
                  />
                </template>
              </Column>
              <Column :header="t('webServer.nativeListenPorts')" style="width: 140px">
                <template #body="{ data }">
                  <template v-if="data.listening_ports && data.listening_ports.length">
                    <FpTag v-for="p in data.listening_ports" :key="p" class="mr-1">{{ p }}</FpTag>
                  </template>
                  <span v-else>—</span>
                </template>
              </Column>
              <Column :header="t('webServer.nativePath')" style="min-width: 160px">
                <template #body="{ data }">
                  <span v-if="data.binary_path" v-tooltip="data.binary_path" class="cell-truncate">{{
                    data.binary_path
                  }}</span>
                  <span v-else>—</span>
                </template>
              </Column>
              <Column :header="t('webServer.actions')" style="width: 180px" frozen>
                <template #body="{ data }">
                  <div class="row-actions">
                    <FpButton
                      v-if="!data.installed"
                      variant="primary"
                      @click="confirmNativeInstall(data)"
                      >{{ t('webServer.nativeInstallBtn') }}</FpButton
                    >
                    <FpButton v-else variant="danger" @click="confirmNativeUninstall(data)">{{
                      t('webServer.nativeUninstallBtn')
                    }}</FpButton>
                  </div>
                </template>
              </Column>
            </FpTable>
          </div>
        </TabPanel>
      </TabPanels>
    </Tabs>

    <FpModal v-model="showInstall" :header="t('webServer.installTitle')" style="width: 500px">
      <div class="modal-form">
        <FpSelect
          v-model="form.engine"
          :label="t('webServer.engine')"
          :options="engines"
          option-label="name"
          option-value="name"
          :invalid="!form.engine"
        />
        <FpInput
          v-model="form.version"
          :label="t('webServer.version')"
          :placeholder="t('webServer.versionPlaceholder')"
        />
        <FpInput
          v-model="form.config_path"
          :label="t('webServer.configPath')"
          :placeholder="t('webServer.configPathPlaceholder')"
        />
        <FpInput
          v-model="form.binary_path"
          :label="t('webServer.binaryPath')"
          :placeholder="t('webServer.binaryPathPlaceholder')"
        />
        <div class="field-col">
          <label class="field-label">{{ t('webServer.port') }}</label>
          <InputNumber v-model="form.port" :min="1" :max="65535" class="w-full" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showInstall = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleInstall">
          {{ t('common.install') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showPreset" :header="t('webServer.preset')" style="width: 520px">
      <div class="modal-form">
        <InlineMessage severity="info" class="preset-alert">{{ t('webServer.presetRecommend') }}</InlineMessage>
        <RadioButtonGroup v-model="selectedPreset" class="preset-group">
          <div v-for="p in presets" :key="p.name" class="preset-radio">
            <RadioButton :value="p.name" :input-id="`preset-${p.name}`" />
            <label :for="`preset-${p.name}`" class="preset-label">
              {{ p.description }}
              <FpTag v-if="p.recommended" severity="success" value="推荐" />
            </label>
          </div>
        </RadioButtonGroup>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showPreset = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleApplyPreset">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showSwitch" :header="t('webServer.switchEngine')" style="width: 480px">
      <div class="modal-form">
        <FpSelect
          v-model="switchEngine"
          :label="t('webServer.engine')"
          :options="engines"
          option-label="name"
          option-value="name"
        />
        <span class="switch-tip">{{ t('webServer.switchTip') }}</span>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showSwitch = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSwitchEngine">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Tabs from 'openvue/tabs'
import TabList from 'openvue/tablist'
import Tab from 'openvue/tab'
import TabPanels from 'openvue/tabpanels'
import TabPanel from 'openvue/tabpanel'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
import InputNumber from 'openvue/inputnumber'
import ToggleSwitch from 'openvue/toggleswitch'
import InlineMessage from 'openvue/inlinemessage'
import RadioButtonGroup from 'openvue/radiobuttongroup'
import RadioButton from 'openvue/radiobutton'
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
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

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
    toast.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
}

async function handleInstall() {
  if (!form.value.engine) {
    toast.warning(t('common.required'))
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
    toast.success(t('common.success'))
    showInstall.value = false
    form.value = { engine: '', version: '', config_path: '', binary_path: '', port: 80 }
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function handleStart(id: number) {
  try {
    await startWebServer(id)
    toast.success(t('common.success'))
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleStop(id: number) {
  try {
    await stopWebServer(id)
    toast.success(t('common.success'))
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleRestart(id: number) {
  try {
    await restartWebServer(id)
    toast.success(t('common.success'))
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleReload(id: number) {
  try {
    await reloadWebServer(id)
    toast.success(t('common.success'))
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleConfigtest(id: number) {
  try {
    await configtestWebServer(id)
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

function confirmDelete(row: WebServerResponse) {
  confirmAction({
    message: t('webServer.deleteConfirm', { name: `${row.engine}-${row.id}` }),
    header: t('common.confirmAction'),
    accept: async () => {
      try {
        await deleteWebServer(row.id)
        toast.success(t('common.success'))
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
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
    toast.error(t('common.failed'))
  }
}

async function handleApplyPreset() {
  if (!selectedPreset.value) return
  submitting.value = true
  try {
    await applyWebServerPreset(targetServerId, selectedPreset.value)
    toast.success(t('common.success'))
    showPreset.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
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
    toast.success(t('common.success'))
    showSwitch.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
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
    toast.error(t('common.failed'))
  } finally {
    detecting.value = false
  }
}

function confirmNativeInstall(row: NativeWebServerInfo) {
  confirmAction({
    message: t('webServer.nativeInstallConfirm', { name: row.engine }),
    header: t('common.confirmAction'),
    accept: async () => {
      if (nativeBusy.value) return
      nativeBusy.value = true
      try {
        await nativeInstallWebServer(row.engine)
        toast.success(t('common.success'))
        await fetchNative()
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      } finally {
        nativeBusy.value = false
      }
    },
  })
}

function confirmNativeUninstall(row: NativeWebServerInfo) {
  confirmAction({
    message: t('webServer.nativeUninstallConfirm', { name: row.engine }),
    header: t('common.confirmAction'),
    accept: async () => {
      if (nativeBusy.value) return
      nativeBusy.value = true
      try {
        await nativeUninstallWebServer(row.engine)
        toast.success(t('common.success'))
        await fetchNative()
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      } finally {
        nativeBusy.value = false
      }
    },
  })
}

async function handleNativeAutostart(row: NativeWebServerInfo, enabled: boolean) {
  try {
    await nativeAutostartWebServer(row.engine, enabled)
    toast.success(t('common.success'))
    await fetchNative()
  } catch {
    toast.error(t('common.failed'))
  }
}

onMounted(() => {
  fetch()
  fetchNative()
})
</script>

<style scoped>
.servers-tabs {
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  padding: var(--fp-space-4);
}
.page-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--fp-space-4);
}
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.native-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  flex-wrap: wrap;
  margin-bottom: var(--fp-space-4);
}
.native-hint {
  flex: 1;
  min-width: 260px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-1);
  flex-wrap: wrap;
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
.cell-truncate {
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mr-1 {
  margin-right: 4px;
}
.preset-alert {
  width: 100%;
}
.preset-group {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.preset-radio {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.preset-label {
  font-size: 13px;
  color: var(--fp-text-primary);
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  cursor: pointer;
}
.switch-tip {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
</style>
