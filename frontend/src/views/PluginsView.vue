<template>
  <div>
    <div style="display:flex;justify-content:space-between;align-items:center">
      <h2>Plugins</h2>
      <el-button type="primary" @click="showLoad = true">Load Plugin</el-button>
    </div>
    <el-table :data="plugins" border stripe v-loading="loading" style="margin-top:16px">
      <el-table-column prop="id" label="ID" width="150" />
      <el-table-column prop="name" label="Name" />
      <el-table-column prop="version" label="Version" width="80" />
      <el-table-column prop="author" label="Author" width="120" />
      <el-table-column prop="status" label="Status" width="100">
        <template #default="{ row }">
          <el-tag :type="row.enabled ? 'success' : 'danger'" effect="plain" size="small">
            {{ row.enabled ? 'Enabled' : 'Disabled' }}
          </el-tag>
          <div style="font-size:11px;color:#909399;margin-top:2px">{{ row.status }}</div>
        </template>
      </el-table-column>
      <el-table-column prop="exec_count" label="Exec" width="60" align="center" />
      <el-table-column label="Actions" width="380" fixed="right">
        <template #default="{ row }">
          <el-button size="small" :disabled="!row.enabled" @click="disablePlugin(row.id)">Disable</el-button>
          <el-button size="small" :disabled="row.enabled" @click="enablePlugin(row.id)">Enable</el-button>
          <el-button size="small" @click="showExecute(row.id)">Execute</el-button>
          <el-popconfirm title="Unload this plugin?" @confirm="unloadPlugin(row.id)">
            <template #reference>
              <el-button size="small" type="danger">Unload</el-button>
            </template>
          </el-popconfirm>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="showLoad" title="Load Plugin" width="520px" destroy-on-close>
      <el-form :model="loadForm" label-width="100px" @submit.prevent="handleLoad">
        <el-form-item label="ID" required>
          <el-input v-model="loadForm.id" placeholder="unique plugin ID" />
        </el-form-item>
        <el-form-item label="Name" required>
          <el-input v-model="loadForm.name" placeholder="display name" />
        </el-form-item>
        <el-form-item label="WASM (base64)" required>
          <el-input v-model="loadForm.wasm" type="textarea" :rows="4" placeholder="base64-encoded wasm bytes" />
        </el-form-item>
        <el-form-item label="Version">
          <el-input v-model="loadForm.version" placeholder="0.1.0" />
        </el-form-item>
        <el-form-item label="Author">
          <el-input v-model="loadForm.author" />
        </el-form-item>
        <el-form-item label="Description">
          <el-input v-model="loadForm.desc" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showLoad = false">Cancel</el-button>
        <el-button type="primary" @click="handleLoad" :loading="loadLoading">Load</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showExec" title="Execute Plugin" width="600px" destroy-on-close>
      <template #header>
        <span>Execute — <code>{{ execId }}</code></span>
      </template>
      <el-form @submit.prevent="handleExec" label-width="100px">
        <el-form-item label="Function">
          <el-input v-model="execFunc" placeholder="run" />
        </el-form-item>
        <el-form-item label="Args (comma separated)">
          <el-input v-model="execArgs" placeholder="e.g. 1,2,3" />
        </el-form-item>
        <el-button type="primary" native-type="submit" :loading="execLoading">Run</el-button>
      </el-form>
      <el-divider />
      <div v-if="execResult" style="margin-top:8px">
        <div style="font-weight:600;margin-bottom:6px;font-size:13px">Response:</div>
        <pre class="exec-output">{{ execResult }}</pre>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { listPlugins, loadPlugin, unloadPlugin as unload, enablePlugin as enable, disablePlugin as disable, executePlugin } from '@/api/plugins'
import { ElMessage } from 'element-plus'
import type { PluginResponse } from '@/types'

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
  try { plugins.value = (await listPlugins()).data } catch { ElMessage.error('Failed to fetch plugins') }
  finally { loading.value = false }
}

async function handleLoad() {
  if (!loadForm.value.id || !loadForm.value.name || !loadForm.value.wasm) {
    ElMessage.warning('ID, Name and WASM are required')
    return
  }
  loadLoading.value = true
  try {
    await loadPlugin(loadForm.value.id, loadForm.value.name, loadForm.value.wasm, {
      version: loadForm.value.version, author: loadForm.value.author, description: loadForm.value.desc,
    })
    ElMessage.success('Plugin loaded')
    showLoad.value = false
    fetch()
  } catch (e: any) { ElMessage.error(e.response?.data?.message || 'Load failed') }
  finally { loadLoading.value = false }
}

async function enablePlugin(id: string) { try { await enable(id); ElMessage.success('Enabled'); fetch() } catch { ElMessage.error('Enable failed') } }
async function disablePlugin(id: string) { try { await disable(id); ElMessage.success('Disabled'); fetch() } catch { ElMessage.error('Disable failed') } }
async function unloadPlugin(id: string) { try { await unload(id); ElMessage.success('Unloaded'); fetch() } catch { ElMessage.error('Unload failed') } }

function showExecute(id: string) {
  execId.value = id; execResult.value = ''; execArgs.value = ''; execFunc.value = 'run'
  showExec.value = true
}

async function handleExec() {
  execLoading.value = true
  execResult.value = ''
  try {
    const args = execArgs.value ? execArgs.value.split(',').map(Number) : undefined
    const res = await executePlugin(execId.value, execFunc.value, args)
    execResult.value = JSON.stringify(res.data, null, 2)
  } catch (e: any) {
    execResult.value = `Error: ${e.response?.data?.message || e.message}`
  }
  finally { execLoading.value = false }
}

onMounted(fetch)
</script>

<style scoped>
.exec-output {
  background: #f5f7fa; padding: 12px; border-radius: 6px;
  font-size: 13px; max-height: 300px; overflow: auto;
  margin: 0; white-space: pre-wrap; word-break: break-all;
}
</style>
