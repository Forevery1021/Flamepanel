<template>
  <LayoutContent :title="t('nav.plugins')" :description="t('pageDesc.plugins')">
    <FpTabs v-model="activeTab" :items="tabItems">
      <template #installed>
        <PageToolbar v-model="searchText">
          <template #left>
            <FpSelect
              v-model="statusFilter"
              :options="statusOptions"
              option-label="label"
              option-value="value"
              show-clear
              class="w-[160px]"
            />
          </template>
          <template #actions>
            <FpButton
              v-permission="{ perm: 'plugin:create', mode: 'view' }"
              variant="primary"
              icon="oi oi-upload"
              @click="showLoad = true"
              >{{ t('plugin.load') }}</FpButton
            >
          </template>
        </PageToolbar>

        <div class="panel">
          <FpTable :rows="filteredPlugins" :loading="loading" :empty-text="t('common.noData')">
            <FpColumn field="id" :header="t('plugin.id')" style="width: 150px" />
            <FpColumn field="name" :header="t('plugin.name')" />
            <FpColumn field="version" :header="t('plugin.version')" style="width: 80px" />
            <FpColumn field="author" :header="t('plugin.author')" style="width: 120px" />
            <FpColumn :header="t('plugin.status')" style="width: 110px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.enabled ? 'success' : 'danger'"
                  :value="data.enabled ? t('plugin.enabled') : t('plugin.disabled')"
                />
                <div class="status-sub">{{ data.status }}</div>
              </template>
            </FpColumn>
            <FpColumn :header="t('plugin.capabilities')" style="width: 200px">
              <template #body="{ data }">
                <CapabilityTags :capabilities="data.capabilities ?? []" />
              </template>
            </FpColumn>
            <FpColumn :header="t('plugin.execCount')" style="width: 60px">
              <template #body="{ data }">
                <div class="center-cell">{{ data.exec_count }}</div>
              </template>
            </FpColumn>
            <FpColumn :header="t('plugin.actions')" style="width: 400px" frozen>
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton
                    v-permission="{ perm: 'plugin:create', mode: 'view' }"
                    variant="ghost"
                    :disabled="!data.enabled"
                    @click="disablePlugin(data.id)"
                  >
                    {{ t('plugin.disable') }}
                  </FpButton>
                  <FpButton
                    v-permission="{ perm: 'plugin:create', mode: 'view' }"
                    variant="ghost"
                    :disabled="data.enabled"
                    @click="enablePlugin(data.id)"
                  >
                    {{ t('plugin.enable') }}
                  </FpButton>
                  <FpButton
                    v-permission="{ perm: 'plugin:execute', mode: 'view' }"
                    variant="ghost"
                    @click="showExecute(data.id)"
                  >
                    {{ t('plugin.execute') }}
                  </FpButton>
                  <FpButton
                    v-permission="{ perm: 'plugin:create', mode: 'view' }"
                    variant="ghost"
                    @click="openCapabilities(data)"
                  >
                    {{ t('plugin.editCapabilities') }}
                  </FpButton>
                  <FpButton
                    v-permission="{ perm: 'plugin:delete', mode: 'view' }"
                    variant="danger"
                    @click="confirmUnload(data)"
                  >
                    {{ t('plugin.unload') }}
                  </FpButton>
                </div>
              </template>
            </FpColumn>
          </FpTable>
        </div>
      </template>
      <template #market>
        <PluginMarketPanel @installed="activeTab = 'installed'" />
      </template>
    </FpTabs>

    <FpModal v-model="showLoad" :header="t('plugin.load')" size="md">
      <div class="modal-form">
        <FpInput
          v-model="loadForm.id"
          :label="t('plugin.id')"
          :placeholder="t('common.placeholder')"
        />
        <FpInput
          v-model="loadForm.name"
          :label="t('plugin.name')"
          :placeholder="t('common.placeholder')"
        />
        <FpTextarea
          v-model="loadForm.wasm"
          :label="t('plugin.wasmBase64')"
          :rows="4"
          :placeholder="t('common.placeholder')"
          class="w-full"
        />
        <FpInput
          v-model="loadForm.version"
          :label="t('plugin.version')"
          :placeholder="t('common.placeholder')"
        />
        <FpInput v-model="loadForm.author" :label="t('plugin.author')" />
        <FpTextarea
          v-model="loadForm.desc"
          :label="t('plugin.description')"
          :rows="2"
          class="w-full"
        />
        <FpTextarea
          v-model="loadForm.caps"
          :label="t('plugin.capabilities')"
          :rows="2"
          class="w-full mono"
          placeholder="metrics:read"
        />
        <div class="hint">{{ t('plugin.capabilitiesHint') }}</div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showLoad = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="loadLoading" @click="handleLoad">
          {{ t('plugin.load') }}
        </FpButton>
      </template>
    </FpModal>

    <PluginExecuteDialog v-model="showExec" :plugin-id="execId" @executed="invalidate" />
    <PluginCapabilitiesDialog v-model="showCaps" :plugin="capsPlugin" @saved="invalidate" />
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  listPlugins,
  loadPlugin,
  unloadPlugin as unload,
  enablePlugin as enable,
  disablePlugin as disable,
} from '@/api/plugins'
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
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import CapabilityTags from '@/components/plugins/CapabilityTags.vue'
import PluginMarketPanel from '@/components/plugins/PluginMarketPanel.vue'
import PluginExecuteDialog from '@/components/plugins/PluginExecuteDialog.vue'
import PluginCapabilitiesDialog from '@/components/plugins/PluginCapabilitiesDialog.vue'
import { parseCapabilityInput } from '@/utils/pluginCapabilities'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import { getErrorMessage } from '@/utils/error'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type { PluginResponse } from '@/types'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()

const activeTab = ref('installed')
const tabItems = computed<FpTabItem[]>(() => [
  { value: 'installed', label: t('plugin.tabInstalled'), icon: 'oi oi-list' },
  { value: 'market', label: t('plugin.tabMarket'), icon: 'oi oi-cloud-download' },
])

// M9：搜索 + 状态筛选（客户端过滤，插件为全量列表）
const searchText = ref('')
const statusFilter = ref<string>('')
const statusOptions = computed(() => [
  { label: t('plugin.enabled'), value: 'enabled' },
  { label: t('plugin.disabled'), value: 'disabled' },
])
const filteredPlugins = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return plugins.value.filter((p) => {
    if (statusFilter.value === 'enabled' && !p.enabled) return false
    if (statusFilter.value === 'disabled' && p.enabled) return false
    if (!kw) return true
    return (
      p.id.toLowerCase().includes(kw) ||
      p.name.toLowerCase().includes(kw) ||
      (p.author ?? '').toLowerCase().includes(kw)
    )
  })
})

const showLoad = ref(false)
const loadLoading = ref(false)
const loadForm = ref({
  id: '',
  name: '',
  wasm: '',
  version: '0.1.0',
  author: '',
  desc: '',
  caps: '',
})

const showExec = ref(false)
const execId = ref('')
const showCaps = ref(false)
const capsPlugin = ref<PluginResponse | null>(null)

// P3-A：插件全量列表走统一数据获取层 useApiQuery
const pluginsQuery = useApiQuery<PluginResponse[]>(
  () => queryKeys.plugins.list(),
  async () => {
    const res = await listPlugins()
    return { data: res.data }
  },
)
const plugins = computed<PluginResponse[]>(() => pluginsQuery.data.value ?? [])
const loading = pluginsQuery.loading

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.plugins.all })
}

async function handleLoad() {
  if (!loadForm.value.id || !loadForm.value.name || !loadForm.value.wasm) {
    toast.warning(t('common.required'))
    return
  }
  loadLoading.value = true
  try {
    await loadPlugin(loadForm.value.id, loadForm.value.name, loadForm.value.wasm, {
      version: loadForm.value.version,
      author: loadForm.value.author,
      description: loadForm.value.desc,
      capabilities: parseCapabilityInput(loadForm.value.caps),
    })
    toast.success(t('common.success'))
    showLoad.value = false
    invalidate()
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('common.failed')))
  } finally {
    loadLoading.value = false
  }
}

async function enablePlugin(id: string) {
  try {
    await enable(id)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function disablePlugin(id: string) {
  try {
    await disable(id)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
  }
}
function confirmUnload(row: PluginResponse) {
  confirmAction({
    message: t('plugin.loadConfirm', { name: row.name || row.id }),
    header: t('common.warning'),
    accept: async () => {
      try {
        await unload(row.id)
        toast.success(t('common.success'))
        invalidate()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
}

function showExecute(id: string) {
  execId.value = id
  showExec.value = true
}

function openCapabilities(row: PluginResponse) {
  capsPlugin.value = row
  showCaps.value = true
}
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.status-sub {
  font-size: 11px;
  color: var(--fp-text-muted);
  margin-top: 2px;
}
.center-cell {
  text-align: center;
}
.hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}
</style>
