<template>
  <div>
    <h2>Docker</h2>
    <el-tabs v-model="tab">
      <el-tab-pane label="Containers" name="containers">
        <el-table :data="containers" border stripe v-loading="loadingC" style="margin-top:8px">
          <el-table-column prop="id" label="ID" width="200" />
          <el-table-column prop="name" label="Name" />
          <el-table-column prop="image" label="Image" />
          <el-table-column prop="status" label="Status" width="100">
            <template #default="{ row }">
              <el-tag :type="row.status === 'running' ? 'success' : 'info'" effect="plain">{{ row.status }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="Actions" width="420" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="startContainer(row.id)" :disabled="row.status === 'running'">Start</el-button>
              <el-button size="small" @click="stopContainer(row.id)" :disabled="row.status !== 'running'">Stop</el-button>
              <el-button size="small" @click="restartContainer(row.id)">Restart</el-button>
              <el-button size="small" @click="viewLogs(row)">Logs</el-button>
              <el-button size="small" @click="viewStats(row)">Stats</el-button>
              <el-popconfirm title="Remove this container?" @confirm="removeContainer(row.id)">
                <template #reference>
                  <el-button size="small" type="danger">Remove</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="Images" name="images">
        <el-table :data="images" border stripe v-loading="loadingI" style="margin-top:8px">
          <el-table-column prop="id" label="ID" width="200" />
          <el-table-column prop="repo_tags" label="Tags" />
          <el-table-column label="Actions" width="120" fixed="right">
            <template #default="{ row }">
              <el-popconfirm title="Delete this image?" @confirm="removeImage(row.id)">
                <template #reference>
                  <el-button size="small" type="danger">Remove</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="Compose" name="compose">
        <el-card shadow="never" style="margin-top:8px">
          <el-form @submit.prevent="deployCompose" label-width="120px">
            <el-form-item label="Project Name">
              <el-input v-model="composeForm.name" placeholder="e.g. myapp" />
            </el-form-item>
            <el-form-item label="Compose YAML">
              <el-input v-model="composeForm.yaml" type="textarea" :rows="12" font-family="monospace" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" native-type="submit" :loading="composeLoading">Deploy</el-button>
              <el-button @click="composeUp(composeForm.name)" :disabled="!composeForm.name">Up</el-button>
              <el-button @click="composeDown(composeForm.name)" :disabled="!composeForm.name">Down</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="showLogs" title="Container Logs" width="900px" top="30px">
      <template #header>
        <span>Container Logs — <code>{{ logContainer }}</code></span>
      </template>
      <div class="log-toolbar">
        <el-select v-model="logTail" size="small" style="width:100px;margin-right:8px">
          <el-option label="50 lines" :value="50" />
          <el-option label="100 lines" :value="100" />
          <el-option label="500 lines" :value="500" />
          <el-option label="All" :value="9999" />
        </el-select>
        <el-button size="small" @click="refreshLogs">Refresh</el-button>
      </div>
      <pre class="log-viewer">{{ logsContent || 'No logs' }}</pre>
    </el-dialog>

    <el-dialog v-model="showStats" title="Container Stats" width="700px">
      <template #header>
        <span>Container Stats — <code>{{ statContainer }}</code></span>
      </template>
      <div v-if="statsData" style="font-family:monospace;font-size:13px;white-space:pre-wrap">{{ JSON.stringify(statsData, null, 2) }}</div>
      <div v-else style="color:#909399">No stats available</div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  listContainers, startContainer as start, stopContainer as stop,
  restartContainer as restart, removeContainer as remove, containerLogs,
  listImages, removeImage as removeImg, composeDeploy,
  composeUp as up, composeDown as down, containerStats,
} from '@/api/docker'
import { ElMessage } from 'element-plus'
import type { DockerContainer } from '@/types'

const tab = ref('containers')
const containers = ref<DockerContainer[]>([])
const images = ref<any[]>([])
const loadingC = ref(false)
const loadingI = ref(false)

const composeLoading = ref(false)
const composeForm = ref({ name: 'myapp', yaml: 'version: "3"\nservices:\n  web:\n    image: nginx:alpine' })

const showLogs = ref(false)
const logContainer = ref('')
const logsContent = ref('')
const logTail = ref(100)

const showStats = ref(false)
const statContainer = ref('')
const statsData = ref<any>(null)

async function fetchContainers() {
  loadingC.value = true
  try { containers.value = (await listContainers()).data } catch { ElMessage.error('Failed to fetch containers') }
  finally { loadingC.value = false }
}

async function fetchImages() {
  loadingI.value = true
  try { images.value = (await listImages()).data } catch { ElMessage.error('Failed to fetch images') }
  finally { loadingI.value = false }
}

async function startContainer(id: string) { try { await start(id); ElMessage.success('Started'); fetchContainers() } catch { ElMessage.error('Start failed') } }
async function stopContainer(id: string) { try { await stop(id); ElMessage.success('Stopped'); fetchContainers() } catch { ElMessage.error('Stop failed') } }
async function restartContainer(id: string) { try { await restart(id); ElMessage.success('Restarted'); fetchContainers() } catch { ElMessage.error('Restart failed') } }
async function removeContainer(id: string) { try { await remove(id); ElMessage.success('Removed'); fetchContainers() } catch { ElMessage.error('Remove failed') } }
async function removeImage(id: string) { try { await removeImg(id); ElMessage.success('Removed'); fetchImages() } catch { ElMessage.error('Remove failed') } }

async function viewLogs(row: DockerContainer) {
  logContainer.value = row.name || row.id
  showLogs.value = true
  await refreshLogs()
}

async function refreshLogs() {
  try {
    const res = await containerLogs(logContainer.value, logTail.value)
    logsContent.value = res.data
  } catch { logsContent.value = 'Failed to load logs' }
}

async function viewStats(row: DockerContainer) {
  statContainer.value = row.name || row.id
  showStats.value = true
  try {
    const res = await containerStats(row.id)
    statsData.value = res.data
  } catch { statsData.value = null }
}

async function deployCompose() {
  if (!composeForm.value.name || !composeForm.value.yaml) {
    ElMessage.warning('Fill in project name and compose YAML')
    return
  }
  composeLoading.value = true
  try {
    await composeDeploy(composeForm.value.name, composeForm.value.yaml)
    ElMessage.success('Compose deployed')
  } catch { ElMessage.error('Deploy failed') }
  finally { composeLoading.value = false }
}
async function composeUp(name: string) { try { await up(name); ElMessage.success('Compose up') } catch { ElMessage.error('Up failed') } }
async function composeDown(name: string) { try { await down(name); ElMessage.success('Compose down') } catch { ElMessage.error('Down failed') } }

onMounted(() => { fetchContainers(); fetchImages() })
</script>

<style scoped>
.log-toolbar { display: flex; align-items: center; margin-bottom: 8px; }
.log-viewer {
  background: #1e1e1e; color: #d4d4d4; padding: 16px; border-radius: 6px;
  max-height: 500px; overflow: auto; font-size: 13px; line-height: 1.5;
  margin: 0; white-space: pre-wrap; word-break: break-all;
}
</style>
