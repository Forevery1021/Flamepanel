<template>
  <div class="view-container">
    <FpTabs v-model="tab" class="docker-tabs" :items="tabItems">
<template #containers>
<div class="toolbar">
            <div class="search-wrap">
              <FpInput
                v-model="searchText"
                :placeholder="t('docker.searchPlaceholder')"
                @update:model-value="applyFilter"
              />
            </div>
            <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" plain icon="oi oi-trash" @click="confirmPrune('containers')">
              {{ t('docker.prune') }}
            </FpButton>
          </div>
          <FpTable
            :rows="filteredContainers"
            :loading="loadingC"
            :paginator="false"
            :empty-text="t('common.noData')"
            striped-rows
            virtual
            virtual-scroll-height="560px"
          >
            <FpColumn field="name" :header="t('docker.name')" style="min-width: 140px" />
            <FpColumn field="id" :header="t('docker.containerId')" style="width: 120px">
              <template #body="{ data }">
                <span v-tooltip="data.id" class="cell-truncate">{{ data.id }}</span>
              </template>
            </FpColumn>
            <FpColumn field="image" :header="t('docker.image')" style="min-width: 140px">
              <template #body="{ data }">
                <span v-tooltip="data.image" class="cell-truncate">{{ data.image }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('docker.status')" style="width: 110px">
              <template #body="{ data }">
                <FpTag
                  :severity="statusType(data.status)"
                  :value="data.status"
                  :dot="data.status === 'running'"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('docker.actions')" style="width: 660px" frozen align-frozen="right">
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton
                    v-permission="{ perm: 'docker:start', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-play-circle"
                    :disabled="data.status === 'running'"
                    @click="startContainer(data.id)"
                    >{{ t('docker.start') }}</FpButton
                  >
                  <FpButton
                    v-permission="{ perm: 'docker:stop', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-stop-circle"
                    :disabled="data.status !== 'running'"
                    @click="stopContainer(data.id)"
                    >{{ t('docker.stop') }}</FpButton
                  >
                  <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }" variant="ghost" icon="oi oi-sync" @click="restartContainer(data.id)">{{
                    t('docker.restart')
                  }}</FpButton>
                  <FpButton variant="ghost" icon="oi oi-terminal" @click="viewLogs(data)">{{
                    t('docker.logs')
                  }}</FpButton>
                  <FpButton variant="ghost" icon="oi oi-gauge" @click="viewStats(data)">{{
                    t('docker.stats')
                  }}</FpButton>
                  <FpButton variant="ghost" icon="oi oi-eye" @click="viewDetail(data)">{{
                    t('docker.inspect')
                  }}</FpButton>
                  <FpButton v-permission="{ perm: 'docker:update', mode: 'view' }" variant="ghost" icon="oi oi-pencil" @click="openRename(data)">{{
                    t('docker.rename')
                  }}</FpButton>
                  <FpButton
                    v-permission="{ perm: 'docker:start', mode: 'view' }"
                    variant="ghost"
                    icon="oi oi-pause-circle"
                    :disabled="data.status !== 'running' && data.status !== 'paused'"
                    @click="togglePause(data)"
                    >{{ data.status === 'paused' ? t('docker.unpause') : t('docker.pause') }}</FpButton
                  >
                  <FpButton
                    v-permission="{ perm: 'docker:start', mode: 'view' }"
                    variant="warning"
                    :disabled="data.status !== 'running'"
                    @click="killContainer(data.id)"
                    >{{ t('docker.kill') }}</FpButton
                  >
                  <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" icon="oi oi-trash" @click="confirmRemoveContainer(data)">{{
                    t('docker.remove')
                  }}</FpButton>
                </div>
              </template>
            </FpColumn>
          </FpTable>
</template>
<template #images>
<div class="toolbar">
            <div class="search-wrap">
              <FpInput
                v-model="pullImageName"
                :placeholder="t('docker.imageNamePlaceholder')"
                @keyup.enter="doPullImage"
              />
            </div>
            <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }" variant="primary" icon="oi oi-download" :loading="pulling" @click="doPullImage">{{
              t('docker.pull')
            }}</FpButton>
            <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" plain icon="oi oi-trash" @click="confirmPrune('images')">
              {{ t('docker.prune') }}
            </FpButton>
          </div>
          <FpTable
            :rows="images"
            :loading="loadingI"
            :paginator="false"
            :empty-text="t('common.noData')"
            striped-rows
            virtual
            virtual-scroll-height="480px"
          >
            <FpColumn field="id" :header="t('docker.containerId')" style="width: 200px" />
            <FpColumn :header="t('docker.tags')" style="min-width: 240px">
              <template #body="{ data }">
                <FpTag
                  v-for="tag in data.tags"
                  :key="tag"
                  severity="neutral"
                  :value="tag"
                  class="mr-1"
                />
                <span v-if="!data.tags || !data.tags.length" class="text-muted">{{
                  data.repo_tags?.join(', ') || data.id
                }}</span>
              </template>
            </FpColumn>
            <FpColumn field="size" :header="t('docker.imageSize')" style="width: 100px">
              <template #body="{ data }">{{ formatSize(data.size) }}</template>
            </FpColumn>
            <FpColumn :header="t('docker.actions')" style="width: 130px" frozen align-frozen="right">
              <template #body="{ data }">
                <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" icon="oi oi-trash" @click="confirmRemoveImage(data)">{{
                  t('docker.remove')
                }}</FpButton>
              </template>
            </FpColumn>
          </FpTable>
</template>
<template #networks>
<div class="toolbar">
            <FpButton v-permission="{ perm: 'docker:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="showCreateNetwork = true">{{
              t('docker.createNetwork')
            }}</FpButton>
            <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" plain icon="oi oi-trash" @click="confirmPrune('networks')">
              {{ t('docker.prune') }}
            </FpButton>
          </div>
          <FpTable
            :rows="networks"
            :loading="loadingN"
            :paginator="false"
            :empty-text="t('common.noData')"
            striped-rows
          >
            <FpColumn field="name" :header="t('docker.name')" style="min-width: 140px" />
            <FpColumn field="driver" :header="t('docker.driver')" style="width: 100px" />
            <FpColumn field="scope" :header="t('docker.scope')" style="width: 90px" />
            <FpColumn :header="t('docker.connectedContainers')" style="min-width: 180px">
              <template #body="{ data }">
                <span v-if="data.containers && data.containers.length">
                  <FpTag
                    v-for="c in data.containers"
                    :key="c.name"
                    severity="neutral"
                    class="mr-1"
                  >
                    {{ c.name }}<span v-if="c.ipv4_address"> ({{ c.ipv4_address }})</span>
                  </FpTag>
                </span>
                <span v-else class="text-muted">—</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('docker.actions')" style="width: 260px" frozen align-frozen="right">
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton variant="ghost" icon="oi oi-link" @click="openConnect(data)">{{
                    t('docker.connect')
                  }}</FpButton>
                  <FpButton variant="ghost" icon="oi oi-minus-circle" @click="openDisconnect(data)">{{
                    t('docker.disconnect')
                  }}</FpButton>
                  <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" icon="oi oi-trash" @click="confirmRemoveNetwork(data)">{{
                    t('docker.remove')
                  }}</FpButton>
                </div>
              </template>
            </FpColumn>
          </FpTable>
</template>
<template #volumes>
<div class="toolbar">
            <FpButton v-permission="{ perm: 'docker:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="showCreateVolume = true">{{
              t('docker.createVolume')
            }}</FpButton>
            <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" plain icon="oi oi-trash" @click="confirmPrune('volumes')">
              {{ t('docker.prune') }}
            </FpButton>
          </div>
          <FpTable
            :rows="volumes"
            :loading="loadingV"
            :paginator="false"
            :empty-text="t('common.noData')"
            striped-rows
            virtual
            virtual-scroll-height="480px"
          >
            <FpColumn field="name" :header="t('docker.name')" style="min-width: 160px" />
            <FpColumn field="driver" :header="t('docker.driver')" style="width: 100px" />
            <FpColumn field="mountpoint" :header="t('docker.mountpoint')" style="min-width: 220px" />
            <FpColumn :header="t('docker.actions')" style="width: 130px" frozen align-frozen="right">
              <template #body="{ data }">
                <FpButton v-permission="{ perm: 'docker:delete', mode: 'view' }" variant="danger" icon="oi oi-trash" @click="confirmRemoveVolume(data)">{{
                  t('docker.remove')
                }}</FpButton>
              </template>
            </FpColumn>
          </FpTable>
</template>
<template #compose>
<div class="panel">
            <form class="modal-form" @submit.prevent="deployCompose">
              <FpInput
                v-model="composeForm.name"
                :label="t('docker.projectName')"
                :placeholder="t('common.placeholder')"
              />
              <div class="field-col">
                <label class="field-label">{{ t('docker.yaml') }}</label>
                <FpTextarea
                  v-model="composeForm.yaml"
                  :rows="12"
                  class="compose-textarea w-full"
                />
              </div>
              <div class="form-actions">
                <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }" variant="primary" type="submit" icon="oi oi-upload" :loading="composeLoading">{{
                  t('docker.deploy')
                }}</FpButton>
                <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }" variant="ghost" :disabled="!composeForm.name" @click="composeUp(composeForm.name)">{{
                  t('docker.up')
                }}</FpButton>
                <FpButton
                  v-permission="{ perm: 'docker:stop', mode: 'view' }"
                  variant="ghost"
                  :disabled="!composeForm.name"
                  @click="composeDown(composeForm.name)"
                  >{{ t('docker.down') }}</FpButton
                >
              </div>
            </form>
            <FpDivider align="left">{{ t('docker.projects') }}</FpDivider>
            <FpTable
              :rows="projects"
              :loading="loadingP"
              :paginator="false"
              :empty-text="t('common.noData')"
              striped-rows
              virtual
              virtual-scroll-height="360px"
            >
              <FpColumn field="name" :header="t('docker.projectName')" style="min-width: 140px" />
              <FpColumn field="status" :header="t('docker.projectStatus')" style="width: 120px" />
              <FpColumn
                field="config_files"
                :header="t('docker.projectConfigFiles')"
                style="min-width: 200px"
              />
              <FpColumn :header="t('docker.actions')" style="width: 180px" frozen align-frozen="right">
                <template #body="{ data }">
                  <div class="row-actions">
                    <FpButton v-permission="{ perm: 'docker:start', mode: 'view' }" variant="ghost" @click="composeUp(data.name)">{{
                      t('docker.up')
                    }}</FpButton>
                    <FpButton v-permission="{ perm: 'docker:stop', mode: 'view' }" variant="danger" @click="composeDown(data.name)">{{
                      t('docker.down')
                    }}</FpButton>
                  </div>
                </template>
              </FpColumn>
            </FpTable>
          </div>
</template>
</FpTabs>

    <!-- 容器日志 -->
    <FpModal
      v-model="showLogs"
      :header="`${t('docker.containerLogs')} — ${logContainer}`"
      style="width: 900px"
    >
      <div class="log-toolbar">
        <div class="tail-select">
          <FpSelect
            v-model="logTail"
            :options="logTailOptions"
            option-label="label"
            option-value="value"
          />
        </div>
        <FpButton variant="ghost" icon="oi oi-refresh" @click="refreshLogs">{{
          t('docker.refresh')
        }}</FpButton>
      </div>
      <pre class="log-viewer">{{ logsContent || t('docker.noLogs') }}</pre>
    </FpModal>

    <!-- 容器统计 -->
    <FpModal
      v-model="showStats"
      :header="`${t('docker.containerStats')} — ${statContainer}`"
      style="width: 700px"
    >
      <div v-if="statsData" class="mono text-xs pre-wrap">
        {{ JSON.stringify(statsData, null, 2) }}
      </div>
      <div v-else class="text-muted">{{ t('docker.noStats') }}</div>
    </FpModal>

    <!-- 容器详情 -->
    <FpModal
      v-model="showDetail"
      :header="`${t('docker.detail')} — ${detailContainer}`"
      style="width: 900px"
    >
      <pre class="log-viewer">{{ detailContent || t('common.noData') }}</pre>
    </FpModal>

    <!-- 重命名 -->
    <FpModal v-model="showRename" :header="t('docker.rename')" style="width: 420px">
      <form class="modal-form" @submit.prevent="doRename">
        <FpInput
          v-model="renameTarget"
          :label="t('docker.name')"
          :error="renameError"
          :placeholder="t('docker.renamePlaceholder')"
        />
      </form>
      <template #footer>
        <FpButton variant="ghost" @click="showRename = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="renaming" @click="doRename">{{
          t('common.confirm')
        }}</FpButton>
      </template>
    </FpModal>

    <!-- 创建网络 -->
    <FpModal v-model="showCreateNetwork" :header="t('docker.createNetwork')" style="width: 480px">
      <form class="modal-form" @submit.prevent="doCreateNetwork">
        <FpInput v-model="networkForm.name" :label="t('docker.name')" :error="networkNameError" />
        <FpSelect
          v-model="networkForm.driver"
          :label="t('docker.driver')"
          :options="networkDriverOptions"
          option-label="label"
          option-value="value"
        />
        <FpInput
          v-model="networkForm.subnet"
          :label="t('docker.subnet')"
          placeholder="172.28.0.0/16"
        />
      </form>
      <template #footer>
        <FpButton variant="ghost" @click="showCreateNetwork = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="creating" @click="doCreateNetwork">{{
          t('common.confirm')
        }}</FpButton>
      </template>
    </FpModal>

    <!-- 创建卷 -->
    <FpModal v-model="showCreateVolume" :header="t('docker.createVolume')" style="width: 480px">
      <form class="modal-form" @submit.prevent="doCreateVolume">
        <FpInput v-model="volumeForm.name" :label="t('docker.name')" :error="volumeNameError" />
        <FpSelect
          v-model="volumeForm.driver"
          :label="t('docker.driver')"
          :options="volumeDriverOptions"
          option-label="label"
          option-value="value"
        />
      </form>
      <template #footer>
        <FpButton variant="ghost" @click="showCreateVolume = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="creating" @click="doCreateVolume">{{
          t('common.confirm')
        }}</FpButton>
      </template>
    </FpModal>

    <!-- 连接容器 -->
    <FpModal v-model="showConnect" :header="t('docker.connectContainer')" style="width: 480px">
      <form class="modal-form" @submit.prevent="doConnect">
        <FpSelect
          v-model="connectContainerId"
          :label="t('docker.container')"
          :options="connectOptions"
          option-label="label"
          option-value="value"
          filter
          :error="connectError"
        />
      </form>
      <template #footer>
        <FpButton variant="ghost" @click="showConnect = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="connecting" @click="doConnect">{{
          t('common.confirm')
        }}</FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
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
import FpTable from '@/components/ui/FpTable.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import type {
  DockerContainer,
  DockerImage,
  DockerNetwork,
  DockerVolume,
  ComposeProject,
} from '@/types'

const { t } = useI18n()

const tabItems: FpTabItem[] = [
  { value: 'containers', label: t('docker.containers') },
  { value: 'images', label: t('docker.images') },
  { value: 'networks', label: t('docker.networks') },
  { value: 'volumes', label: t('docker.volumes') },
  { value: 'compose', label: t('docker.compose') },
]
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
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
const renameError = ref('')
const renaming = ref(false)
let renameContainerId = ''

const pullImageName = ref('')
const pulling = ref(false)

const showCreateNetwork = ref(false)
const networkForm = ref({ name: '', driver: 'bridge', subnet: '' })
const networkNameError = ref('')
const showCreateVolume = ref(false)
const volumeForm = ref({ name: '', driver: 'local' })
const volumeNameError = ref('')
const creating = ref(false)

const showConnect = ref(false)
const connectContainerId = ref('')
const connectError = ref('')
const connecting = ref(false)
let connectNetworkId = ''

const logTailOptions = computed(() => [
  { label: `50 ${t('docker.tailLines')}`, value: 50 },
  { label: `100 ${t('docker.tailLines')}`, value: 100 },
  { label: `500 ${t('docker.tailLines')}`, value: 500 },
  { label: t('docker.tailLines'), value: 9999 },
])

const networkDriverOptions = [
  { label: 'bridge', value: 'bridge' },
  { label: 'host', value: 'host' },
  { label: 'overlay', value: 'overlay' },
  { label: 'macvlan', value: 'macvlan' },
]

const volumeDriverOptions = [
  { label: 'local', value: 'local' },
  { label: 'nfs', value: 'nfs' },
]

const connectOptions = computed(() =>
  containers.value.map((c) => ({ label: c.name || c.id, value: c.id })),
)

function statusType(status: string): 'success' | 'warning' | 'danger' | 'info' {
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
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    loadingC.value = false
  }
}

async function fetchImages() {
  loadingI.value = true
  try {
    images.value = (await listImages()).data
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    loadingI.value = false
  }
}

async function fetchNetworks() {
  loadingN.value = true
  try {
    networks.value = (await listNetworks()).data
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    loadingN.value = false
  }
}

async function fetchVolumes() {
  loadingV.value = true
  try {
    volumes.value = (await listVolumes()).data
  } catch (e) {
    toast.error(e, t('common.failed'))
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
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function stopContainer(id: string) {
  try {
    await stopApi(id)
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function restartContainer(id: string) {
  try {
    await restartApi(id)
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function removeContainer(id: string) {
  try {
    await removeApi(id)
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function killContainer(id: string) {
  try {
    await killApi(id)
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function togglePause(row: DockerContainer) {
  try {
    if (row.status === 'paused') {
      await unpauseApi(row.id)
    } else {
      await pauseApi(row.id)
    }
    toast.success(t('common.success'))
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
function openRename(row: DockerContainer) {
  renameContainerId = row.id
  renameTarget.value = row.name
  renameError.value = ''
  showRename.value = true
}
async function doRename() {
  if (!renameTarget.value.trim()) {
    renameError.value = t('common.required')
    return
  }
  renaming.value = true
  try {
    await renameApi(renameContainerId, renameTarget.value.trim())
    toast.success(t('common.success'))
    showRename.value = false
    fetchContainers()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    renaming.value = false
  }
}
async function removeImage(id: string) {
  try {
    await removeImgApi(id)
    toast.success(t('common.success'))
    fetchImages()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function doPullImage() {
  if (!pullImageName.value.trim()) return
  pulling.value = true
  try {
    const res = await pullApi(pullImageName.value.trim())
    toast.success(typeof res.data === 'string' ? res.data : t('common.success'))
    pullImageName.value = ''
    fetchImages()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    pulling.value = false
  }
}

function confirmPrune(kind: 'containers' | 'images' | 'networks' | 'volumes') {
  const whatMap: Record<string, string> = {
    containers: t('docker.containers'),
    images: t('docker.images'),
    networks: t('docker.networks'),
    volumes: t('docker.volumes'),
  }
  confirmAction({
    message: t('docker.pruneConfirm', { what: whatMap[kind] }),
    header: t('common.confirmAction'),
    accept: () => prune(kind),
  })
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
    toast.success(t('common.success'))
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}

function confirmRemoveContainer(row: DockerContainer) {
  confirmAction({
    message: t('docker.removeConfirm', { name: row.name || row.id }),
    header: t('common.confirmAction'),
    accept: () => removeContainer(row.id),
  })
}

function confirmRemoveImage(row: DockerImage) {
  confirmAction({
    message: t('docker.removeConfirm', { name: row.id }),
    header: t('common.confirmAction'),
    accept: () => removeImage(row.id),
  })
}

function confirmRemoveNetwork(row: DockerNetwork) {
  confirmAction({
    message: t('docker.removeConfirm', { name: row.name }),
    header: t('common.confirmAction'),
    accept: () => removeNetwork(row.id),
  })
}

function confirmRemoveVolume(row: DockerVolume) {
  confirmAction({
    message: t('docker.removeConfirm', { name: row.name }),
    header: t('common.confirmAction'),
    accept: () => removeVolume(row.name),
  })
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
    toast.warning(t('common.required'))
    return
  }
  composeLoading.value = true
  try {
    await composeDeploy(composeForm.value.name, composeForm.value.yaml)
    toast.success(t('common.success'))
    fetchProjects()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    composeLoading.value = false
  }
}

async function composeUp(name: string) {
  try {
    await upApi(name)
    toast.success(t('common.success'))
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
async function composeDown(name: string) {
  try {
    await downApi(name)
    toast.success(t('common.success'))
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}

async function doCreateNetwork() {
  if (!networkForm.value.name.trim()) {
    networkNameError.value = t('common.required')
    return
  }
  creating.value = true
  try {
    await createNetworkApi(
      networkForm.value.name.trim(),
      networkForm.value.driver,
      networkForm.value.subnet || undefined,
    )
    toast.success(t('common.success'))
    showCreateNetwork.value = false
    networkForm.value = { name: '', driver: 'bridge', subnet: '' }
    fetchNetworks()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    creating.value = false
  }
}
async function removeNetwork(id: string) {
  try {
    await removeNetworkApi(id)
    toast.success(t('common.success'))
    fetchNetworks()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}
function openConnect(row: DockerNetwork) {
  connectNetworkId = row.id
  connectContainerId.value = ''
  connectError.value = ''
  showConnect.value = true
}
async function doConnect() {
  if (!connectContainerId.value) {
    connectError.value = t('common.required')
    return
  }
  connecting.value = true
  try {
    await connectNetwork(connectNetworkId, connectContainerId.value)
    toast.success(t('common.success'))
    showConnect.value = false
    fetchNetworks()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    connecting.value = false
  }
}
async function openDisconnect(row: DockerNetwork) {
  const attached = row.containers || []
  if (!attached.length) {
    toast.info(t('docker.noContainers'))
    return
  }
  const containerName = attached[0].name
  try {
    await disconnectNetwork(row.id, containerName)
    toast.success(t('common.success'))
    fetchNetworks()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}

async function doCreateVolume() {
  if (!volumeForm.value.name.trim()) {
    volumeNameError.value = t('common.required')
    return
  }
  creating.value = true
  try {
    await createVolumeApi(volumeForm.value.name.trim(), volumeForm.value.driver)
    toast.success(t('common.success'))
    showCreateVolume.value = false
    volumeForm.value = { name: '', driver: 'local' }
    fetchVolumes()
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    creating.value = false
  }
}
async function removeVolume(name: string) {
  try {
    await removeVolumeApi(name)
    toast.success(t('common.success'))
    fetchVolumes()
  } catch (e) {
    toast.error(e, t('common.failed'))
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
.docker-tabs {
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  padding: var(--fp-space-4);
}
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
  flex-wrap: wrap;
}
.search-wrap {
  flex: 1;
  max-width: 320px;
  min-width: 220px;
}
.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.form-actions {
  display: flex;
  gap: var(--fp-space-2);
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
.compose-textarea {
  font-family: var(--fp-font-mono);
}
.cell-truncate {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.log-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-3);
}
.tail-select {
  width: 220px;
}
.log-viewer {
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
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
</style>
