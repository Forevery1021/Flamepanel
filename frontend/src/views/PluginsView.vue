<template>
  <div class="view-container">
    <div class="page-toolbar">
      <FpButton variant="primary" icon="oi oi-upload" @click="showLoad = true">{{ t('plugin.load') }}</FpButton>
    </div>

    <div class="panel">
      <FpTable :rows="plugins" :loading="loading" :empty-text="t('common.noData')">
        <Column field="id" :header="t('plugin.id')" style="width: 150px" />
        <Column field="name" :header="t('plugin.name')" />
        <Column field="version" :header="t('plugin.version')" style="width: 80px" />
        <Column field="author" :header="t('plugin.author')" style="width: 120px" />
        <Column :header="t('plugin.status')" style="width: 110px">
          <template #body="{ data }">
            <FpTag
              :severity="data.enabled ? 'success' : 'danger'"
              :value="data.enabled ? t('plugin.enabled') : t('plugin.disabled')"
            />
            <div class="status-sub">{{ data.status }}</div>
          </template>
        </Column>
        <Column :header="t('plugin.execCount')" style="width: 60px">
          <template #body="{ data }">
            <div class="center-cell">{{ data.exec_count }}</div>
          </template>
        </Column>
        <Column :header="t('plugin.actions')" style="width: 400px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="ghost" :disabled="!data.enabled" @click="disablePlugin(data.id)">
                {{ t('plugin.disable') }}
              </FpButton>
              <FpButton variant="ghost" :disabled="data.enabled" @click="enablePlugin(data.id)">
                {{ t('plugin.enable') }}
              </FpButton>
              <FpButton variant="ghost" @click="showExecute(data.id)">
                {{ t('plugin.execute') }}
              </FpButton>
              <FpButton variant="danger" @click="confirmUnload(data)">
                {{ t('plugin.unload') }}
              </FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
    </div>

    <FpModal v-model="showLoad" :header="t('plugin.load')" style="width: 520px">
      <div class="modal-form">
        <FpInput v-model="loadForm.id" :label="t('plugin.id')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="loadForm.name" :label="t('plugin.name')" :placeholder="t('common.placeholder')" />
        <div class="field-col">
          <label class="field-label">{{ t('plugin.wasmBase64') }}</label>
          <Textarea v-model="loadForm.wasm" :rows="4" :placeholder="t('common.placeholder')" class="w-full" />
        </div>
        <FpInput v-model="loadForm.version" :label="t('plugin.version')" :placeholder="t('common.placeholder')" />
        <FpInput v-model="loadForm.author" :label="t('plugin.author')" />
        <div class="field-col">
          <label class="field-label">{{ t('plugin.description') }}</label>
          <Textarea v-model="loadForm.desc" :rows="2" class="w-full" />
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
      <Divider />
      <div v-if="execResult">
        <div class="response-title">{{ t('plugin.response') }}:</div>
        <pre class="exec-output">{{ execResult }}</pre>
      </div>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Textarea from 'openvue/textarea'
import Divider from 'openvue/divider'
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
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { getErrorMessage } from '@/utils/error'
import type { PluginResponse } from '@/types'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const plugins = ref<PluginResponse[]>([])
const loading = ref(false)

const showLoad = ref(false)
const loadLoading = ref(false)
const loadForm = ref({ id: '', name: '', wasm: '', version: '0.1.0', author: '', desc: '' })

const showExec = ref(false)
const execId = ref('')
const execFunc = ref('run')
const execArgs = ref('')
const execResult = ref('')
const execLoading = ref(false)

async function fetch() {
  loading.value = true
  try {
    plugins.value = (await listPlugins()).data
  } catch {
    toast.error(t('common.failed'))
  } finally {
    loading.value = false
  }
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
    fetch()
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
    fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}
async function disablePlugin(id: string) {
  try {
    await disable(id)
    toast.success(t('common.success'))
    fetch()
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
        fetch()
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

onMounted(fetch)
</script>

<style scoped>
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
