<template>
  <LayoutContent :title="t('nav.webServers')" :description="t('pageDesc.webServers')">
    <FpTabs v-model="activeTab" class="servers-tabs" :items="tabItems">
      <template #instances>
        <PageToolbar :searchable="false">
          <template #actions>
            <FpButton
              v-permission="{ perm: 'web_server:create', mode: 'view' }"
              variant="primary"
              icon="oi oi-plus"
              @click="showInstall = true"
            >
              {{ t('webServer.install') }}
            </FpButton>
          </template>
        </PageToolbar>

        <div class="panel">
          <FpTable
            :rows="instances"
            :loading="loading"
            :empty-text="t('common.noData')"
            :first="first"
            striped-rows
          >
            <FpColumn field="id" :header="t('webServer.id')" style="width: 60px" />
            <FpColumn field="engine" :header="t('webServer.engine')" style="width: 120px" />
            <FpColumn :header="t('webServer.version')" style="width: 100px">
              <template #body="{ data }">{{ data.version || '—' }}</template>
            </FpColumn>
            <FpColumn field="port" :header="t('webServer.port')" style="width: 80px" />
            <FpColumn :header="t('webServer.status')" style="width: 110px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.status === 'running' ? 'success' : 'info'"
                  :dot="data.status === 'running'"
                >
                  {{ data.status === 'running' ? t('webServer.running') : t('webServer.stopped') }}
                </FpTag>
              </template>
            </FpColumn>
            <FpColumn :header="t('webServer.configPath')" style="min-width: 160px">
              <template #body="{ data }">
                <span v-tooltip="data.config_path" class="cell-truncate">{{
                  data.config_path
                }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('webServer.actions')" style="width: 240px" frozen>
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton
                    v-if="data.status !== 'running'"
                    v-permission="{ perm: 'web_server:start', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-play-circle"
                    @click="handleStart(data.id)"
                    >{{ t('webServer.start') }}</FpButton
                  >
                  <FpButton
                    v-else
                    v-permission="{ perm: 'web_server:start', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-stop-circle"
                    @click="handleStop(data.id)"
                    >{{ t('webServer.stop') }}</FpButton
                  >
                  <FpButton
                    v-permission="{ perm: 'web_server:reload', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-refresh"
                    @click="handleReload(data.id)"
                    >{{ t('webServer.reload') }}</FpButton
                  >
                  <FpActionMenu :items="rowActions(data)" />
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
            @update:first="onFirst"
          />
        </div>
      </template>
      <template #native>
        <WebServerNativeTab @changed="invalidate" />
      </template>
    </FpTabs>

    <FpModal v-model="showInstall" :header="t('webServer.installTitle')" size="md">
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
          <FpNumber v-model="form.port" :min="1" :max="65535" class="w-full" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showInstall = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleInstall">
          {{ t('common.install') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showPreset" :header="t('webServer.preset')" size="md">
      <div class="modal-form">
        <FpInlineMessage severity="info" class="preset-alert">{{
          t('webServer.presetRecommend')
        }}</FpInlineMessage>
        <FpRadioGroup v-model="selectedPreset" class="preset-group">
          <div v-for="p in presets" :key="p.name" class="preset-radio">
            <FpRadioOption :value="p.name" :input-id="`preset-${p.name}`" />
            <label :for="`preset-${p.name}`" class="preset-label">
              {{ p.description }}
              <FpTag v-if="p.recommended" severity="success" :value="t('common.recommended')" />
            </label>
          </div>
        </FpRadioGroup>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showPreset = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleApplyPreset">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showSwitch" :header="t('webServer.switchEngine')" size="md">
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
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

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
} from '@/api/webServers'
import type { EngineInfo, WebServerResponse, PerformancePresetInfo } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpActionMenu, { type FpActionItem } from '@/components/ui/FpActionMenu.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpRadioGroup from '@/components/ui/FpRadioGroup.vue'
import FpRadioOption from '@/components/ui/FpRadioOption.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { useCrudPage } from '@/composables/useCrudPage'
import { queryKeys } from '@/api/queryKeys'
import WebServerNativeTab from '@/components/webserver/WebServerNativeTab.vue'

const { t } = useI18n()

const tabItems: FpTabItem[] = [
  { value: 'instances', label: t('webServer.instances') },
  { value: 'native', label: t('webServer.nativeTab') },
]
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()
const activeTab = ref('instances')
// Y9：分页状态收敛到 useCrudPage
const crud = useCrudPage({ pageSize: 20 })
const { total, first, pageSize, onFirst } = crud
const submitting = ref(false)
const showInstall = ref(false)
const form = ref({ engine: '', version: '', config_path: '', binary_path: '', port: 80 })

// P3-A：Web 服务器实例与引擎列表分别走统一数据获取层 useApiQuery
const instancesQuery = useApiQuery<{ data: WebServerResponse[]; total: number }>(
  () => queryKeys.webServers.list(crud.currentPage.value, crud.pageSize.value),
  async () => {
    const res = await listWebServers(crud.currentPage.value, crud.pageSize.value)
    crud.total.value = res.data.total
    return { data: { data: res.data.data, total: res.data.total } }
  },
  { keepPrevious: true },
)
const instances = computed<WebServerResponse[]>(() => instancesQuery.data.value?.data ?? [])
const loading = instancesQuery.loading

const enginesQuery = useApiQuery<EngineInfo[]>(
  () => queryKeys.engines.list(),
  async () => {
    const res = await listEngines().catch(() => ({ data: [] as EngineInfo[] }))
    return { data: res.data }
  },
)
const engines = computed<EngineInfo[]>(() => enginesQuery.data.value ?? [])

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.webServers.all })
  queryClient.invalidateQueries({ queryKey: queryKeys.engines.all })
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
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

/** 行操作：启停 / 重载为按钮，其余收进「更多」 */
function rowActions(row: WebServerResponse): FpActionItem[] {
  return [
    {
      key: 'restart',
      label: t('webServer.restart'),
      icon: 'oi oi-sync',
      perm: 'web_server:start',
      command: () => handleRestart(row.id),
    },
    {
      key: 'configtest',
      label: t('webServer.configtest'),
      icon: 'oi oi-check-circle',
      perm: 'web_server:configtest',
      command: () => handleConfigtest(row.id),
    },
    {
      key: 'preset',
      label: t('webServer.preset'),
      icon: 'oi oi-sliders',
      perm: 'web_server:update',
      command: () => openPreset(row),
    },
    {
      key: 'switch',
      label: t('webServer.switchEngine'),
      icon: 'oi oi-random',
      perm: 'web_server:update',
      command: () => openSwitch(row),
    },
    {
      key: 'uninstall',
      label: t('webServer.uninstall'),
      icon: 'oi oi-trash',
      perm: 'web_server:delete',
      danger: true,
      command: () => confirmDelete(row),
    },
  ]
}

async function handleStart(id: number) {
  try {
    await startWebServer(id)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleStop(id: number) {
  try {
    await stopWebServer(id)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleRestart(id: number) {
  try {
    await restartWebServer(id)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function handleReload(id: number) {
  try {
    await reloadWebServer(id)
    toast.success(t('common.success'))
    invalidate()
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
        invalidate()
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
    invalidate()
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
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.servers-tabs {
  background: var(--fp-bg-surface);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  padding: var(--fp-space-4);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-1);
  flex-wrap: wrap;
}
.cell-truncate {
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
