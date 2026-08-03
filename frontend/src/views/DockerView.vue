<template>
  <div class="view-container">
    <div class="card-header-title"></div>
    <el-tabs v-model="tab">
      <!-- ── 容器 ── -->
      <el-tab-pane :label="t('docker.containers')" name="containers">
        <div class="toolbar">
          <el-input
            v-model="searchText"
            :placeholder="t('docker.searchPlaceholder')"
            clearable
            size="small"
            class="search-input"
            @input="applyFilter"
          />
          <el-popconfirm
            :title="t('docker.pruneConfirm', { what: t('docker.containers') })"
            @confirm="prune('containers')"
          >
            <template #reference>
              <el-button size="small" type="danger" plain>{{ t('docker.prune') }}</el-button>
            </template>
          </el-popconfirm>
        </div>
        <el-table
          v-loading="loadingC"
          :empty-text="t('common.noData')"
          :data="filteredContainers"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="name" :label="t('docker.name')" min-width="140" />
          <el-table-column prop="id" :label="t('docker.containerId')" width="120" show-overflow-tooltip />
          <el-table-column prop="image" :label="t('docker.image')" min-width="140" show-overflow-tooltip />
          <el-table-column :label="t('docker.status')" width="100">
            <template #default="{ row }">
              <el-tag :type="statusType(row.status)" effect="plain">{{ row.status }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('docker.actions')" width="560" fixed="right">
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
              <el-button size="small" @click="viewDetail(row)">{{ t('docker.inspect') }}</el-button>
              <el-button size="small" @click="openRename(row)">{{ t('docker.rename') }}</el-button>
              <el-button
                size="small"
                :disabled="row.status !== 'running' && row.status !== 'paused'"
                @click="togglePause(row)"
                >{{ row.status === 'paused' ? t('docker.unpause') : t('docker.pause') }}</el-button
              >
              <el-button
                size="small"
                type="warning"
                :disabled="row.status !== 'running'"
                @click="killContainer(row.id)"
                >{{ t('docker.kill') }}</el-button
              >
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

      <!-- ── 镜像 ── -->
      <el-tab-pane :label="t('docker.images')" name="images">
        <div class="toolbar">
          <el-input
            v-model="pullImageName"
            :placeholder="t('docker.imageNamePlaceholder')"
            clearable
            size="small"
            class="search-input"
            @keyup.enter="doPullImage"
          />
          <el-button size="small" type="primary" :loading="pulling" @click="doPullImage">{{
            t('docker.pull')
          }}</el-button>
          <el-popconfirm
            :title="t('docker.pruneConfirm', { what: t('docker.images') })"
            @confirm="prune('images')"
          >
            <template #reference>
              <el-button size="small" type="danger" plain>{{ t('docker.prune') }}</el-button>
            </template>
          </el-popconfirm>
        </div>
        <el-table
          v-loading="loadingI"
          :empty-text="t('common.noData')"
          :data="images"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="id" :label="t('docker.containerId')" width="200" />
          <el-table-column prop="tags" :label="t('docker.tags')">
            <template #default="{ row }">
              <el-tag v-for="tag in row.tags" :key="tag" size="small" class="mr-1">{{ tag }}</el-tag>
              <span v-if="!row.tags || !row.tags.length" class="text-muted">{{
                row.repo_tags?.join(', ') || row.id
              }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="size" :label="t('docker.imageSize')" width="100">
            <template #default="{ row }">{{ formatSize(row.size) }}</template>
          </el-table-column>
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

      <!-- ── 网络 ── -->
      <el-tab-pane :label="t('docker.networks')" name="networks">
        <div class="toolbar">
          <el-button size="small" type="primary" @click="showCreateNetwork = true">{{
            t('docker.createNetwork')
          }}</el-button>
          <el-popconfirm
            :title="t('docker.pruneConfirm', { what: t('docker.networks') })"
            @confirm="prune('networks')"
          >
            <template #reference>
              <el-button size="small" type="danger" plain>{{ t('docker.prune') }}</el-button>
            </template>
          </el-popconfirm>
        </div>
        <el-table
          v-loading="loadingN"
          :empty-text="t('common.noData')"
          :data="networks"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="name" :label="t('docker.name')" min-width="140" />
          <el-table-column prop="driver" :label="t('docker.driver')" width="100" />
          <el-table-column prop="scope" :label="t('docker.scope')" width="90" />
          <el-table-column :label="t('docker.connectedContainers')" min-width="180">
            <template #default="{ row }">
              <span v-if="row.containers && row.containers.length">
                <el-tag
                  v-for="c in row.containers"
                  :key="c.name"
                  size="small"
                  class="mr-1"
                  effect="plain"
                >
                  {{ c.name }}<span v-if="c.ipv4_address"> ({{ c.ipv4_address }})</span>
                </el-tag>
              </span>
              <span v-else class="text-muted">—</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('docker.actions')" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="openConnect(row)">{{
                t('docker.connect')
              }}</el-button>
              <el-button size="small" @click="openDisconnect(row)">{{
                t('docker.disconnect')
              }}</el-button>
              <el-popconfirm
                :title="t('docker.removeConfirm', { name: row.name })"
                @confirm="removeNetwork(row.id)"
              >
                <template #reference>
                  <el-button size="small" type="danger">{{ t('docker.remove') }}</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- ── 卷 ── -->
      <el-tab-pane :label="t('docker.volumes')" name="volumes">
        <div class="toolbar">
          <el-button size="small" type="primary" @click="showCreateVolume = true">{{
            t('docker.createVolume')
          }}</el-button>
          <el-popconfirm
            :title="t('docker.pruneConfirm', { what: t('docker.volumes') })"
            @confirm="prune('volumes')"
          >
            <template #reference>
              <el-button size="small" type="danger" plain>{{ t('docker.prune') }}</el-button>
            </template>
          </el-popconfirm>
        </div>
        <el-table
          v-loading="loadingV"
          :empty-text="t('common.noData')"
          :data="volumes"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="name" :label="t('docker.name')" min-width="160" />
          <el-table-column prop="driver" :label="t('docker.driver')" width="100" />
          <el-table-column prop="mountpoint" :label="t('docker.mountpoint')" min-width="220" />
          <el-table-column :label="t('docker.actions')" width="120" fixed="right">
            <template #default="{ row }">
              <el-popconfirm
                :title="t('docker.removeConfirm', { name: row.name })"
                @confirm="removeVolume(row.name)"
              >
                <template #reference>
                  <el-button size="small" type="danger">{{ t('docker.remove') }}</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- ── Compose ── -->
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
        <el-divider content-position="left">{{ t('docker.projects') }}</el-divider>
        <el-table
          v-loading="loadingP"
          :empty-text="t('common.noData')"
          :data="projects"
          border
          stripe
          class="mt-2"
        >
          <el-table-column prop="name" :label="t('docker.projectName')" min-width="140" />
          <el-table-column prop="status" :label="t('docker.projectStatus')" width="120" />
          <el-table-column
            prop="config_files"
            :label="t('docker.projectConfigFiles')"
            min-width="200"
          />
          <el-table-column :label="t('docker.actions')" width="180" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="composeUp(row.name)">{{
                t('docker.up')
              }}</el-button>
              <el-button size="small" type="danger" @click="composeDown(row.name)">{{
                t('docker.down')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <!-- 容器日志 -->
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

    <!-- 容器统计 -->
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

    <!-- 容器详情 -->
    <el-dialog v-model="showDetail" :title="t('docker.detail')" width="900px" top="30px">
      <template #header>
        <span>{{ t('docker.detail') }} — <code>{{ detailContainer }}</code></span>
      </template>
      <pre class="log-viewer">{{ detailContent || t('common.noData') }}</pre>
    </el-dialog>

    <!-- 重命名 -->
    <el-dialog v-model="showRename" :title="t('docker.rename')" width="420px">
      <el-form label-width="100px" @submit.prevent="doRename">
        <el-form-item :label="t('docker.name')" required>
          <el-input v-model="renameTarget" :placeholder="t('docker.renamePlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRename = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="renaming" @click="doRename">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 创建网络 -->
    <el-dialog v-model="showCreateNetwork" :title="t('docker.createNetwork')" width="480px">
      <el-form label-width="100px">
        <el-form-item :label="t('docker.name')" required>
          <el-input v-model="networkForm.name" />
        </el-form-item>
        <el-form-item :label="t('docker.driver')">
          <el-select v-model="networkForm.driver" class="full-width">
            <el-option label="bridge" value="bridge" />
            <el-option label="host" value="host" />
            <el-option label="overlay" value="overlay" />
            <el-option label="macvlan" value="macvlan" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('docker.subnet')">
          <el-input v-model="networkForm.subnet" placeholder="172.28.0.0/16" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateNetwork = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="creating" @click="doCreateNetwork">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 创建卷 -->
    <el-dialog v-model="showCreateVolume" :title="t('docker.createVolume')" width="480px">
      <el-form label-width="100px">
        <el-form-item :label="t('docker.name')" required>
          <el-input v-model="volumeForm.name" />
        </el-form-item>
        <el-form-item :label="t('docker.driver')">
          <el-select v-model="volumeForm.driver" class="full-width">
            <el-option label="local" value="local" />
            <el-option label="nfs" value="nfs" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateVolume = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="creating" @click="doCreateVolume">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 连接容器 -->
    <el-dialog v-model="showConnect" :title="t('docker.connectContainer')" width="480px">
      <el-form label-width="100px">
        <el-form-item :label="t('docker.container')" required>
          <el-select v-model="connectContainerId" class="full-width" filterable>
            <el-option
              v-for="c in containers"
              :key="c.id"
              :label="c.name || c.id"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showConnect = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="connecting" @click="doConnect">{{
          t('common.confirm')
        }}</el-button>
      </template>
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
  renameContainer as renameApi,
  pauseContainer as pauseApi,
  unpauseContainer as unpauseApi,
  killContainer as killApi,
  pruneContainers,
  containerLogs,
  listImages,
  removeImage as removeImgApi,
  pullImage as pullApi,
  pruneImages,
  composeDeploy,
  composeUp as upApi,
  composeDown as downApi,
  containerStats,
  inspectContainer,
  listNetworks,
  createNetwork as createNetworkApi,
  removeNetwork as removeNetworkApi,
  connectNetwork,
  disconnectNetwork,
  pruneNetworks,
  listVolumes,
  createVolume as createVolumeApi,
  removeVolume as removeVolumeApi,
  pruneVolumes,
  listComposeProjects,
} from '@/api/docker'
import { ElMessage } from 'element-plus'
import type {
  DockerContainer,
  DockerImage,
  DockerNetwork,
  DockerVolume,
  ComposeProject,
} from '@/types'

const { t } = useI18n()
const tab = ref('containers')

// ── 容器 ──
const containers = ref<DockerContainer[]>([])
const filteredContainers = ref<DockerContainer[]>([])
const searchText = ref('')
const loadingC = ref(false)

const images = ref<DockerImage[]>([])
const loadingI = ref(false)

const networks = ref<DockerNetwork[]>([])
const loadingN = ref(false)

const volumes = ref<DockerVolume[]>([])
const loadingV = ref(false)

const projects = ref<ComposeProject[]>([])
const loadingP = ref(false)

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

const showDetail = ref(false)
const detailContainer = ref('')
const detailContent = ref('')

const showRename = ref(false)
const renameTarget = ref('')
const renaming = ref(false)
let renameContainerId = ''

const pullImageName = ref('')
const pulling = ref(false)

const showCreateNetwork = ref(false)
const networkForm = ref({ name: '', driver: 'bridge', subnet: '' })
const showCreateVolume = ref(false)
const volumeForm = ref({ name: '', driver: 'local' })
const creating = ref(false)

const showConnect = ref(false)
const connectContainerId = ref('')
const connecting = ref(false)
let connectNetworkId = ''

function statusType(status: string) {
  if (status === 'running') return 'success'
  if (status === 'paused') return 'warning'
  if (status === 'exited' || status === 'dead') return 'danger'
  return 'info'
}

function formatSize(bytes: number) {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let v = bytes
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(1)} ${units[i]}`
}

function applyFilter() {
  const q = searchText.value.trim().toLowerCase()
  if (!q) {
    filteredContainers.value = containers.value
    return
  }
  filteredContainers.value = containers.value.filter(
    (c) =>
      c.name.toLowerCase().includes(q) ||
      c.id.toLowerCase().includes(q) ||
      c.image.toLowerCase().includes(q),
  )
}

async function fetchContainers() {
  loadingC.value = true
  try {
    containers.value = (await listContainers()).data
    applyFilter()
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

async function fetchNetworks() {
  loadingN.value = true
  try {
    networks.value = (await listNetworks()).data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loadingN.value = false
  }
}

async function fetchVolumes() {
  loadingV.value = true
  try {
    volumes.value = (await listVolumes()).data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loadingV.value = false
  }
}

async function fetchProjects() {
  loadingP.value = true
  try {
    projects.value = (await listComposeProjects()).data
  } catch {
    // 无 docker compose CLI 时静默忽略
    projects.value = []
  } finally {
    loadingP.value = false
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
async function killContainer(id: string) {
  try {
    await killApi(id)
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
async function togglePause(row: DockerContainer) {
  try {
    if (row.status === 'paused') {
      await unpauseApi(row.id)
    } else {
      await pauseApi(row.id)
    }
    ElMessage.success(t('common.success'))
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
function openRename(row: DockerContainer) {
  renameContainerId = row.id
  renameTarget.value = row.name
  showRename.value = true
}
async function doRename() {
  if (!renameTarget.value.trim()) return
  renaming.value = true
  try {
    await renameApi(renameContainerId, renameTarget.value.trim())
    ElMessage.success(t('common.success'))
    showRename.value = false
    fetchContainers()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    renaming.value = false
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
async function doPullImage() {
  if (!pullImageName.value.trim()) return
  pulling.value = true
  try {
    const res = await pullApi(pullImageName.value.trim())
    ElMessage.success(typeof res.data === 'string' ? res.data : t('common.success'))
    pullImageName.value = ''
    fetchImages()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    pulling.value = false
  }
}

async function prune(kind: 'containers' | 'images' | 'networks' | 'volumes') {
  try {
    switch (kind) {
      case 'containers':
        await pruneContainers()
        fetchContainers()
        break
      case 'images':
        await pruneImages()
        fetchImages()
        break
      case 'networks':
        await pruneNetworks()
        fetchNetworks()
        break
      case 'volumes':
        await pruneVolumes()
        fetchVolumes()
        break
    }
    ElMessage.success(t('common.success'))
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

async function viewDetail(row: DockerContainer) {
  detailContainer.value = row.name || row.id
  showDetail.value = true
  detailContent.value = t('common.loading')
  try {
    const res = await inspectContainer(row.id)
    detailContent.value = JSON.stringify(res.data, null, 2)
  } catch {
    detailContent.value = t('common.failed')
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
    fetchProjects()
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

async function doCreateNetwork() {
  if (!networkForm.value.name.trim()) return
  creating.value = true
  try {
    await createNetworkApi(
      networkForm.value.name.trim(),
      networkForm.value.driver,
      networkForm.value.subnet || undefined,
    )
    ElMessage.success(t('common.success'))
    showCreateNetwork.value = false
    networkForm.value = { name: '', driver: 'bridge', subnet: '' }
    fetchNetworks()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    creating.value = false
  }
}
async function removeNetwork(id: string) {
  try {
    await removeNetworkApi(id)
    ElMessage.success(t('common.success'))
    fetchNetworks()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}
function openConnect(row: DockerNetwork) {
  connectNetworkId = row.id
  connectContainerId.value = ''
  showConnect.value = true
}
async function doConnect() {
  if (!connectContainerId.value) return
  connecting.value = true
  try {
    await connectNetwork(connectNetworkId, connectContainerId.value)
    ElMessage.success(t('common.success'))
    showConnect.value = false
    fetchNetworks()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    connecting.value = false
  }
}
async function openDisconnect(row: DockerNetwork) {
  const attached = row.containers || []
  if (!attached.length) {
    ElMessage.info(t('docker.noContainers'))
    return
  }
  const containerName = attached[0].name
  try {
    await disconnectNetwork(row.id, containerName)
    ElMessage.success(t('common.success'))
    fetchNetworks()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function doCreateVolume() {
  if (!volumeForm.value.name.trim()) return
  creating.value = true
  try {
    await createVolumeApi(volumeForm.value.name.trim(), volumeForm.value.driver)
    ElMessage.success(t('common.success'))
    showCreateVolume.value = false
    volumeForm.value = { name: '', driver: 'local' }
    fetchVolumes()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    creating.value = false
  }
}
async function removeVolume(name: string) {
  try {
    await removeVolumeApi(name)
    ElMessage.success(t('common.success'))
    fetchVolumes()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

onMounted(() => {
  fetchContainers()
  fetchImages()
  fetchNetworks()
  fetchVolumes()
  fetchProjects()
})
</script>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  flex-wrap: wrap;
}
.search-input {
  max-width: 320px;
}
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
.mr-1 {
  margin-right: 4px;
}
.mt-2 {
  margin-top: 8px;
}
.full-width {
  width: 100%;
}
</style>
