<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.plugins') }}</h2>
      <el-button type="primary" @click="showLoad = true">{{ t('plugin.load') }}</el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :empty-text="t('common.noData')"
        :data="plugins"
        border
        stripe
        class="full-width"
      >
        <el-table-column prop="id" :label="t('plugin.id')" width="150" />
        <el-table-column prop="name" :label="t('plugin.name')" />
        <el-table-column prop="version" :label="t('plugin.version')" width="80" />
        <el-table-column prop="author" :label="t('plugin.author')" width="120" />
        <el-table-column :label="t('plugin.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" effect="plain" size="small">
              {{ row.enabled ? t('plugin.enabled') : t('plugin.disabled') }}
            </el-tag>
            <div class="text-11 text-muted mt-1">{{ row.status }}</div>
          </template>
        </el-table-column>
        <el-table-column
          prop="exec_count"
          :label="t('plugin.execCount')"
          width="60"
          align="center"
        />
        <el-table-column :label="t('plugin.actions')" width="380" fixed="right">
          <template #default="{ row }">
            <el-button size="small" :disabled="!row.enabled" @click="disablePlugin(row.id)">{{
              t('plugin.disable')
            }}</el-button>
            <el-button size="small" :disabled="row.enabled" @click="enablePlugin(row.id)">{{
              t('plugin.enable')
            }}</el-button>
            <el-button size="small" @click="showExecute(row.id)">{{
              t('plugin.execute')
            }}</el-button>
            <el-popconfirm
              :title="t('plugin.loadConfirm', { name: row.name || row.id })"
              @confirm="unloadPlugin(row.id)"
            >
              <template #reference>
                <el-button size="small" type="danger">{{ t('plugin.unload') }}</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="showLoad" :title="t('plugin.load')" width="520px" destroy-on-close>
      <el-form :model="loadForm" label-width="100px" @submit.prevent="handleLoad">
        <el-form-item :label="t('plugin.id')" required>
          <el-input v-model="loadForm.id" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-form-item :label="t('plugin.name')" required>
          <el-input v-model="loadForm.name" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-form-item :label="t('plugin.wasmBase64')" required>
          <el-input
            v-model="loadForm.wasm"
            type="textarea"
            :rows="4"
            :placeholder="t('common.placeholder')"
          />
        </el-form-item>
        <el-form-item :label="t('plugin.version')">
          <el-input v-model="loadForm.version" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-form-item :label="t('plugin.author')">
          <el-input v-model="loadForm.author" />
        </el-form-item>
        <el-form-item :label="t('plugin.description')">
          <el-input v-model="loadForm.desc" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showLoad = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="loadLoading" @click="handleLoad">{{
          t('plugin.load')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showExec" :title="t('plugin.execute')" width="600px" destroy-on-close>
      <template #header>
        <span
          >{{ t('plugin.execute') }} — <code>{{ execId }}</code></span
        >
      </template>
      <el-form label-width="100px" @submit.prevent="handleExec">
        <el-form-item :label="t('plugin.function')">
          <el-input v-model="execFunc" placeholder="run" />
        </el-form-item>
        <el-form-item :label="t('plugin.args')">
          <el-input v-model="execArgs" :placeholder="t('common.placeholder')" />
        </el-form-item>
        <el-button type="primary" native-type="submit" :loading="execLoading">{{
          t('plugin.run')
        }}</el-button>
      </el-form>
      <el-divider />
      <div v-if="execResult" class="mt-2">
        <div class="font-semibold mb-1">{{ t('plugin.response') }}:</div>
        <pre class="exec-output">{{ execResult }}</pre>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  listPlugins,
  loadPlugin,
  unloadPlugin as unload,
  enablePlugin as enable,
  disablePlugin as disable,
  executePlugin,
} from '@/api/plugins'
import { ElMessage } from 'element-plus'
import { getErrorMessage } from '@/utils/error'
import type { PluginResponse } from '@/types'

const { t } = useI18n()
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
    ElMessage.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function handleLoad() {
  if (!loadForm.value.id || !loadForm.value.name || !loadForm.value.wasm) {
    ElMessage.warning(t('common.required'))
    return
  }
  loadLoading.value = true
  try {
    await loadPlugin(loadForm.value.id, loadForm.value.name, loadForm.value.wasm, {
      version: loadForm.value.version,
      author: loadForm.value.author,
      description: loadForm.value.desc,
    })
    ElMessage.success(t('common.success'))
    showLoad.value = false
    fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loadLoading.value = false
  }
}

async function enablePlugin(id: string) {
  try {
    await enable(id)
    ElMessage.success(t('common.success'))
    fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function disablePlugin(id: string) {
  try {
    await disable(id)
    ElMessage.success(t('common.success'))
    fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function unloadPlugin(id: string) {
  try {
    await unload(id)
    ElMessage.success(t('common.success'))
    fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
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
.exec-output {
  background: #f5f7fa;
  padding: 12px;
  border-radius: 6px;
  font-size: 13px;
  max-height: 300px;
  overflow: auto;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
