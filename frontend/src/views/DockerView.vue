<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.docker') }}</h2>
    </div>
    <el-tabs v-model="tab">
      <el-tab-pane :label="t('docker.containers')" name="containers">
        <el-table
          v-loading="loadingC"
          :empty-text="t('common.noData')"
          :data="containers"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="id" :label="t('docker.containerId')" width="200" />
          <el-table-column prop="name" :label="t('docker.name')" />
          <el-table-column prop="image" :label="t('docker.image')" />
          <el-table-column :label="t('docker.status')" width="100">
            <template #default="{ row }">
              <el-tag :type="row.status === 'running' ? 'success' : 'info'" effect="plain">{{
                row.status
              }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('docker.actions')" width="420" fixed="right">
            <template #default="{ row }">
              <el-button
                size="small"
                :disabled="row.status === 'running'"
                @click="startContainer(row.id)"
                >{{ t('docker.start') }}</el-button
              >
              <el-button
                size="small"
                :disabled="row.status !== 'running'"
                @click="stopContainer(row.id)"
                >{{ t('docker.stop') }}</el-button
              >
              <el-button size="small" @click="restartContainer(row.id)">{{
                t('docker.restart')
              }}</el-button>
              <el-button size="small" @click="viewLogs(row)">{{ t('docker.logs') }}</el-button>
              <el-button size="small" @click="viewStats(row)">{{ t('docker.stats') }}</el-button>
              <el-popconfirm
                :title="t('docker.removeConfirm', { name: row.name || row.id })"
                @confirm="removeContainer(row.id)"
              >
                <template #reference>
                  <el-button size="small" type="danger">{{ t('docker.remove') }}</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane :label="t('docker.images')" name="images">
        <el-table
          v-loading="loadingI"
          :empty-text="t('common.noData')"
          :data="images"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="id" :label="t('docker.containerId')" width="200" />
          <el-table-column prop="repo_tags" :label="t('docker.tags')" />
          <el-table-column :label="t('docker.actions')" width="120" fixed="right">
            <template #default="{ row }">
              <el-popconfirm
                :title="t('docker.removeConfirm', { name: row.id })"
                @confirm="removeImage(row.id)"
              >
                <template #reference>
                  <el-button size="small" type="danger">{{ t('docker.remove') }}</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane :label="t('docker.compose')" name="compose">
        <el-card shadow="never" class="mt-2">
          <el-form label-width="120px" @submit.prevent="deployCompose">
            <el-form-item :label="t('docker.projectName')">
              <el-input v-model="composeForm.name" :placeholder="t('common.placeholder')" />
            </el-form-item>
            <el-form-item :label="t('docker.yaml')">
              <el-input
                v-model="composeForm.yaml"
                type="textarea"
                :rows="12"
                font-family="monospace"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" native-type="submit" :loading="composeLoading">{{
                t('docker.deploy')
              }}</el-button>
              <el-button :disabled="!composeForm.name" @click="composeUp(composeForm.name)">{{
                t('docker.up')
              }}</el-button>
              <el-button :disabled="!composeForm.name" @click="composeDown(composeForm.name)">{{
                t('docker.down')
              }}</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="showLogs" :title="t('docker.containerLogs')" width="900px" top="30px">
      <template #header>
        <span
          >{{ t('docker.containerLogs') }} — <code>{{ logContainer }}</code></span
        >
      </template>
      <div class="log-toolbar">
        <el-select v-model="logTail" size="small" class="w-100 mr-2">
          <el-option :label="'50 ' + t('docker.tailLines')" :value="50" />
          <el-option :label="'100 ' + t('docker.tailLines')" :value="100" />
          <el-option :label="'500 ' + t('docker.tailLines')" :value="500" />
          <el-option :label="t('docker.tailLines')" :value="9999" />
        </el-select>
        <el-button size="small" @click="refreshLogs">{{ t('docker.refresh') }}</el-button>
      </div>
      <pre class="log-viewer">{{ logsContent || t('docker.noLogs') }}</pre>
    </el-dialog>

    <el-dialog v-model="showStats" :title="t('docker.containerStats')" width="700px">
      <template #header>
        <span
          >{{ t('docker.containerStats') }} — <code>{{ statContainer }}</code></span
        >
      </template>
      <div v-if="statsData" class="mono text-xs pre-wrap">
        {{ JSON.stringify(statsData, null, 2) }}
      </div>
      <div v-else class="text-muted">{{ t('docker.noStats') }}</div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  listContainers,
  startContainer as startApi,
  stopContainer as stopApi,
  restartContainer as restartApi,
  removeContainer as removeApi,
  containerLogs,
  listImages,
  removeImage as removeImgApi,
  composeDeploy,
  composeUp as upApi,
  composeDown as downApi,
  containerStats,
} from '@/api/docker'
import { ElMessage } from 'element-plus'
import type { DockerContainer, DockerImage } from '@/types'

const { t } = useI18n()
const tab = ref('containers')
const containers = ref<DockerContainer[]>([])
const images = ref<DockerImage[]>([])
const loadingC = ref(false)
const loadingI = ref(false)

const composeLoading = ref(false)
const composeForm = ref({
  name: 'myapp',
  yaml: 'version: "3"\nservices:\n  web:\n    image: nginx:alpine',
})

const showLogs = ref(false)
const logContainer = ref('')
const logsContent = ref('')
const logTail = ref(100)

const showStats = ref(false)
const statContainer = ref('')
const statsData = ref<unknown>(null)

async function fetchContainers() {
  loadingC.value = true
  try {
    containers.value = (await listContainers()).data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loadingC.value = false
  }
}

async function fetchImages() {
  loadingI.value = true
  try {
    images.value = (await listImages()).data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loadingI.value = false
  }
}

async function startContainer(id: string) {
  try {
    await startApi(id)
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function stopContainer(id: string) {
  try {
    await stopApi(id)
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function restartContainer(id: string) {
  try {
    await restartApi(id)
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function removeContainer(id: string) {
  try {
    await removeApi(id)
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function removeImage(id: string) {
  try {
    await removeImgApi(id)
    ElMessage.success(t('common.success'))
    fetchImages()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function viewLogs(row: DockerContainer) {
  logContainer.value = row.name || row.id
  showLogs.value = true
  await refreshLogs()
}

async function refreshLogs() {
  try {
    const res = await containerLogs(logContainer.value, logTail.value)
    logsContent.value = res.data
  } catch {
    logsContent.value = t('common.failed')
  }
}

async function viewStats(row: DockerContainer) {
  statContainer.value = row.name || row.id
  showStats.value = true
  try {
    const res = await containerStats(row.id)
    statsData.value = res.data
  } catch {
    statsData.value = null
  }
}

async function deployCompose() {
  if (!composeForm.value.name || !composeForm.value.yaml) {
    ElMessage.warning(t('common.required'))
    return
  }
  composeLoading.value = true
  try {
    await composeDeploy(composeForm.value.name, composeForm.value.yaml)
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    composeLoading.value = false
  }
}

async function composeUp(name: string) {
  try {
    await upApi(name)
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function composeDown(name: string) {
  try {
    await downApi(name)
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

onMounted(() => {
  fetchContainers()
  fetchImages()
})
</script>

<style scoped>
.log-toolbar {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}
.log-viewer {
  background: #1e1e1e;
  color: #d4d4d4;
  padding: 16px;
  border-radius: 6px;
  max-height: 500px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.5;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
