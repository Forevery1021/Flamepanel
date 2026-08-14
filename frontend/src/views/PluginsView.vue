<template>
  <div class="view-container">
    <div class="page-toolbar">
      <div class="toolbar-left">
        <FpInput
          v-model="searchText"
          :placeholder="t('common.searchPlaceholder')"
          class="toolbar-search"
        />
        <FpSelect
          v-model="statusFilter"
          :options="statusOptions"
          option-label="label"
          option-value="value"
          show-clear
          class="toolbar-filter"
        />
      </div>
      <FpButton v-permission="{ perm: 'plugin:create', mode: 'view' }" variant="primary" icon="oi oi-upload" @click="showLoad = true">{{ t('plugin.load') }}</FpButton>
    </div>

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
        <FpColumn :header="t('plugin.execCount')" style="width: 60px">
          <template #body="{ data }">
            <div class="center-cell">{{ data.exec_count }}</div>
          </template>
        </FpColumn>
        <FpColumn :header="t('plugin.actions')" style="width: 400px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton v-permission="{ perm: 'plugin:create', mode: 'view' }" variant="ghost" :disabled="!data.enabled" @click="disablePlugin(data.id)">
                {{ t('plugin.disable') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'plugin:create', mode: 'view' }" variant="ghost" :disabled="data.enabled" @click="enablePlugin(data.id)">
                {{ t('plugin.enable') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'plugin:execute', mode: 'view' }" variant="ghost" @click="showExecute(data.id)">
                {{ t('plugin.execute') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'plugin:delete', mode: 'view' }" variant="danger" @click="confirmUnload(data)">
                {{ t('plugin.unload') }}
              </FpButton>
            </div>
          </template>
        </FpColumn>
      </FpTable>
    </div>

    <FpModal v-model="showLoad" :header="t('plugin.load')" style="width: 520px">
      <div class="modal-form">
        <FpInput v-model="loadForm.id" :label="t('plugin.id')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="loadForm.name" :label="t('plugin.name')" :placeholder="t('common.placeholder')" />
        <div class="field-col">
          <label class="field-label">{{ t('plugin.wasmBase64') }}</label>
          <FpTextarea v-model="loadForm.wasm" :rows="4" :placeholder="t('common.placeholder')" class="w-full" />
        </div>
        <FpInput v-model="loadForm.version" :label="t('plugin.version')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="loadForm.author" :label="t('plugin.author')" />
        <div class="field-col">
          <label class="field-label">{{ t('plugin.description') }}</label>
          <FpTextarea v-model="loadForm.desc" :rows="2" class="w-full" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showLoad = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="loadLoading" @click="handleLoad">
          {{ t('plugin.load') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showExec" :header="`${t('plugin.execute')} — ${execId}`" style="width: 600px">
      <div class="modal-form">
        <FpInput v-model="execFunc" :label="t('plugin.function')" placeholder="run" />
        <FpInput v-model="execArgs" :label="t('plugin.args')" :placeholder="t('common.placeholder')" />
        <div>
          <FpButton variant="primary" :loading="execLoading" @click="handleExec">
            {{ t('plugin.run') }}
          </FpButton>
        </div>
      </div>
      <FpDivider />
      <div v-if="execResult">
        <div class="response-title">{{ t('plugin.response') }}:</div>
        <pre class="exec-output">{{ execResult }}</pre>
      </div>
    </FpModal>
  </div>
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
  executePlugin,
} from '@/api/plugins'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import { getErrorMessage } from '@/utils/error'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type { PluginResponse } from '@/types'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()

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
const loadForm = ref({ id: '', name: '', wasm: '', version: '0.1.0', author: '', desc: '' })

const showExec = ref(false)
const execId = ref('')
const execFunc = ref('run')
const execArgs = ref('')
const execResult = ref('')
const execLoading = ref(false)

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
    })
    toast.success(t('common.success'))
    showLoad.value = false
    invalidate()
  } catch {
    toast.error(t('common.failed'))
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
  execResult.value = ''
  execArgs.value = ''
  execFunc.value = 'run'
  showExec.value = true
}

async function handleExec() {
  execLoading.value = true
  execResult.value = ''
  try {
    const args = execArgs.value ? execArgs.value.split(',').map(Number) : undefined
    const res = await executePlugin(execId.value, execFunc.value, args)
    execResult.value = JSON.stringify(res.data, null, 2)
  } catch (e: unknown) {
    execResult.value = `Error: ${getErrorMessage(e, 'unknown error')}`
  } finally {
    execLoading.value = false
  }
}

</script>

<style scoped>
.page-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.toolbar-search {
  width: 240px;
}
.toolbar-filter {
  width: 160px;
}
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
.field-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.response-title {
  font-weight: 600;
  margin-bottom: var(--fp-space-2);
  color: var(--fp-text-primary);
}
.exec-output {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  font-size: 13px;
  font-family: var(--fp-font-mono);
  max-height: 300px;
  overflow: auto;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
