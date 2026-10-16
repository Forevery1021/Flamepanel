<template>
  <LayoutContent :title="t('nav.docker')" :description="t('pageDesc.docker')">
    <FpTabs v-model="tab" class="docker-tabs" :items="tabItems">
      <template #containers>
        <ContainerListPanel
          v-model:search="containerSearch"
          :rows="containerFiltered"
          :loading="containerLoading"
          @start="containerStart"
          @stop="containerStop"
          @restart="containerRestart"
          @kill="containerKill"
          @pause="containerTogglePause"
          @remove="containerRemove"
          @rename="renameDialog?.open($event)"
          @logs="openLogs"
          @stats="openStats"
          @detail="openDetail"
          @export-compose="onExportCompose"
          @prune="containerPrune"
        />
      </template>

      <template #images>
        <ImageListPanel
          v-model:pull-name="imagePullName"
          :rows="images"
          :loading="imageLoading"
          :pulling="imagePulling"
          @pull="onPull"
          @remove="imageRemove"
          @prune="imagePrune"
        />
      </template>

      <template #networks>
        <NetworkListPanel
          :rows="networks"
          :loading="networkLoading"
          @create="createNetworkDialog?.open()"
          @connect="connectDialog?.open($event)"
          @disconnect="networkDisconnect"
          @remove="networkRemove"
          @prune="networkPrune"
        />
      </template>

      <template #volumes>
        <VolumeListPanel
          :rows="volumes"
          :loading="volumeLoading"
          @create="createVolumeDialog?.open()"
          @remove="volumeRemove"
          @prune="volumePrune"
        />
      </template>

      <template #compose>
        <ComposePanel
          v-model:name="composeName"
          v-model:yaml="composeYaml"
          :rows="projects"
          :loading="composeLoading"
          :submitting="composeSubmitting"
          :warnings="composeWarnings"
          @deploy="onDeploy"
          @import-run="runImportDialog?.open()"
          @up="composeUp"
          @down="composeDown"
        />
      </template>
    </FpTabs>

    <ContainerLogsDialog
      v-model="logsVisible"
      v-model:tail="logTail"
      :container="logContainer"
      :content="logsContent"
      @refresh="refreshLogs"
    />

    <ContainerJsonDialog
      v-model="statsVisible"
      header-key="docker.containerStats"
      :container="statContainer"
      :json="statsJson"
      :empty-text="t('docker.noStats')"
      width="700px"
    />

    <ContainerJsonDialog
      v-model="detailVisible"
      header-key="docker.detail"
      :container="detailContainer"
      :json="detailContent"
      :empty-text="t('common.noData')"
      width="900px"
      variant="terminal"
    />

    <RenameDialog ref="renameDialog" :submitting="renaming" @submit="onRename" />

    <CreateNetworkDialog
      ref="createNetworkDialog"
      :submitting="networkCreating"
      @submit="onCreateNetwork"
    />

    <CreateVolumeDialog
      ref="createVolumeDialog"
      :submitting="volumeCreating"
      @submit="onCreateVolume"
    />

    <ConnectContainerDialog ref="connectDialog" :options="connectCandidates" @submit="onConnect" />
    <RunImportDialog ref="runImportDialog" :submitting="composeConverting" @submit="onImportRun" />
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { connectContainerOptions } from '@/utils/dockerFormat'
import { useDockerContainers } from '@/composables/useDockerContainers'
import { useDockerImages } from '@/composables/useDockerImages'
import { useDockerNetworks } from '@/composables/useDockerNetworks'
import { useDockerVolumes } from '@/composables/useDockerVolumes'
import { useDockerCompose, DEFAULT_COMPOSE_YAML } from '@/composables/useDockerCompose'
import type { ComposeConversion, DockerContainer } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import ContainerListPanel from '@/components/docker/ContainerListPanel.vue'
import ImageListPanel from '@/components/docker/ImageListPanel.vue'
import NetworkListPanel from '@/components/docker/NetworkListPanel.vue'
import VolumeListPanel from '@/components/docker/VolumeListPanel.vue'
import ComposePanel from '@/components/docker/ComposePanel.vue'
import ContainerLogsDialog from '@/components/docker/ContainerLogsDialog.vue'
import ContainerJsonDialog from '@/components/docker/ContainerJsonDialog.vue'
import RenameDialog from '@/components/docker/RenameDialog.vue'
import CreateNetworkDialog from '@/components/docker/CreateNetworkDialog.vue'
import CreateVolumeDialog from '@/components/docker/CreateVolumeDialog.vue'
import ConnectContainerDialog from '@/components/docker/ConnectContainerDialog.vue'
import RunImportDialog from '@/components/docker/RunImportDialog.vue'

/**
 * Docker 管理视图（Doc/23 §3.3 T2-4 巨型视图拆分第三站）。
 *
 * 拆分前 **1107 行**、五个页签（容器 / 镜像 / 网络 / 卷 / Compose）各自携带表格、
 * 弹窗与请求编排。本次沿用 AppStoreView（第一步）与 WebsitesView（第二步）
 * 已验证的四层拆法：
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `useDocker{Containers,Images,Networks,Volumes,Compose}` | 列表查询与惰性启用、行操作容错壳、二次确认、弹窗数据 |
 * | 展示 | `components/docker/*`（10 个） | 纯装配，操作全部上抛 |
 * | 纯函数 | `utils/dockerFormat.ts` | 状态映射、搜索过滤、三处下拉候选共用 |
 * | 装配 | 本文件 | 页签骨架 + 惰性加载编排 + composable ↔ 组件接线 |
 *
 * **与前三站的关键差异**：本站拆出的五个 composable 各自独立持有自己的
 * `enabled` 开关，而「首次切到某页签才加载、切回不重复请求」这条编排只能留在
 * 装配层——它同时依赖页签状态与五个数据层的开关，放进任一 composable 都会让
 * 数据层反过来知道页签的存在。故这是本层唯一的非平凡逻辑，有专门用例锁死。
 *
 * 行为契约不变：同一批 API、同一文案 key、同一确认分支与禁用条件、同一惰性策略。
 */
const { t } = useI18n()

const tabItems = computed<FpTabItem[]>(() => [
  { value: 'containers', label: t('docker.containers') },
  { value: 'images', label: t('docker.images') },
  { value: 'networks', label: t('docker.networks') },
  { value: 'volumes', label: t('docker.volumes') },
  { value: 'compose', label: t('docker.compose') },
])

const tab = ref('containers')

/* ── 五个数据层 ── */

const {
  containers: containerRows,
  filtered: containerFiltered,
  loading: containerLoading,
  search: containerSearch,
  start: containerStart,
  stop: containerStop,
  restart: containerRestart,
  remove: containerRemove,
  kill: containerKill,
  togglePause: containerTogglePause,
  renaming,
  rename,
  logsContent,
  statsData,
  detailContent,
  loadLogs,
  loadStats,
  loadDetail,
  prune: containerPrune,
} = useDockerContainers()

const {
  images: imageRows,
  loading: imageLoading,
  enabled: imageEnabled,
  pullName: imagePullName,
  pulling: imagePulling,
  pull,
  remove: imageRemove,
  prune: imagePrune,
} = useDockerImages()

const {
  networks: networkRows,
  loading: networkLoading,
  enabled: networkEnabled,
  creating: networkCreating,
  create: createNetwork,
  connect: networkConnect,
  disconnect: networkDisconnect,
  remove: networkRemove,
  prune: networkPrune,
} = useDockerNetworks()

const {
  volumes: volumeRows,
  loading: volumeLoading,
  enabled: volumeEnabled,
  creating: volumeCreating,
  create: createVolume,
  remove: volumeRemove,
  prune: volumePrune,
} = useDockerVolumes()

const {
  projects: composeRows,
  loading: composeLoading,
  enabled: composeEnabled,
  submitting: composeSubmitting,
  deploy,
  converting: composeConverting,
  convertRun,
  exportContainer,
  up: composeUp,
  down: composeDown,
} = useDockerCompose()

/** 列表数据统一归一：`useApiQuery` 的 data 在首次加载完成前为 undefined */
const images = computed(() => imageRows.value ?? [])
const networks = computed(() => networkRows.value ?? [])
const volumes = computed(() => volumeRows.value ?? [])
const projects = computed(() => composeRows.value ?? [])

/** 容器列表用于「连接容器」下拉：只读快照，故直接消费其缓存 data */
const connectCandidates = computed(() => connectContainerOptions(containerRows.value ?? []))

const renameDialog = ref<InstanceType<typeof RenameDialog> | null>(null)
const createNetworkDialog = ref<InstanceType<typeof CreateNetworkDialog> | null>(null)
const createVolumeDialog = ref<InstanceType<typeof CreateVolumeDialog> | null>(null)
const connectDialog = ref<InstanceType<typeof ConnectContainerDialog> | null>(null)
const runImportDialog = ref<InstanceType<typeof RunImportDialog> | null>(null)

/* ── 容器页签：日志 / 统计 / 详情三个只读弹窗的编排 ── */

const logsVisible = ref(false)
const logContainer = ref('')
const logTail = ref(100)

/** 打开日志：标题与寻址同用一个标识（名称回退 ID）——与原实现一致 */
function openLogs(row: DockerContainer) {
  logContainer.value = row.name || row.id
  logsVisible.value = true
  void loadLogs(logContainer.value, logTail.value)
}

/** 刷新日志：沿用当前行数（切换行数后点刷新才生效——与原实现一致） */
function refreshLogs() {
  void loadLogs(logContainer.value, logTail.value)
}

const statsVisible = ref(false)
const statContainer = ref('')
/** 统计以 2 空格缩进渲染（后端返回对象；空则显示占位） */
const statsJson = computed(() => (statsData.value ? JSON.stringify(statsData.value, null, 2) : ''))

function openStats(row: DockerContainer) {
  statContainer.value = row.name || row.id
  statsVisible.value = true
  void loadStats(row.id)
}

const detailVisible = ref(false)
const detailContainer = ref('')

function openDetail(row: DockerContainer) {
  detailContainer.value = row.name || row.id
  detailVisible.value = true
  void loadDetail(row.id)
}

/** 重命名：成功才关弹窗（失败保持打开让用户改输入） */
async function onRename(id: string, name: string) {
  if (await rename(id, name)) renameDialog.value?.close()
}

/* ── 镜像 / 网络 / 卷 / Compose 的「成功才关」与表单复位 ── */

/** 拉取镜像：成功后清空输入框（原实现在请求成功后 `pullImageName = ''`） */
async function onPull() {
  if (await pull()) imagePullName.value = ''
}

async function onCreateNetwork(payload: { name: string; driver: string; subnet: string }) {
  if (await createNetwork(payload)) createNetworkDialog.value?.closeAndReset()
}

async function onCreateVolume(payload: { name: string; driver: string }) {
  if (await createVolume(payload)) createVolumeDialog.value?.closeAndReset()
}

async function onConnect(networkId: string, containerId: string) {
  if (await networkConnect(networkId, containerId)) connectDialog.value?.close()
}

/* ── Compose 部署表单（初值是一段可用 YAML，与原实现同源） ── */

const composeName = ref('myapp')
const composeYaml = ref(DEFAULT_COMPOSE_YAML)

/** 最近一次转换的提示：随表单回填展示，部署成功后清空 */
const composeWarnings = ref<string[]>([])

async function onDeploy() {
  if (await deploy({ name: composeName.value, yaml: composeYaml.value })) {
    composeWarnings.value = []
  }
}

/** 转换结果只回填表单（不部署），并切到 Compose 页签供审阅 */
function applyConversion(result: ComposeConversion) {
  composeName.value = result.project_name
  composeYaml.value = result.compose_yaml
  composeWarnings.value = result.warnings
  tab.value = 'compose'
}

/** docker run 粘贴导入：成功才关弹窗（失败保持打开让用户改命令） */
async function onImportRun(command: string) {
  const result = await convertRun(command)
  if (result) {
    applyConversion(result)
    runImportDialog.value?.close()
  }
}

/** 存量容器收编：导出为 Compose 后到 Compose 页签审阅 */
async function onExportCompose(row: DockerContainer) {
  const result = await exportContainer(row.id)
  if (result) applyConversion(result)
}

/* ── 惰性加载编排：每个页签只加载一次，切回不重复请求 ── */

const loadedTabs = new Set<string>(['containers'])

watch(tab, (cur) => {
  if (loadedTabs.has(cur)) return
  loadedTabs.add(cur)
  if (cur === 'images') imageEnabled.value = true
  else if (cur === 'networks') networkEnabled.value = true
  else if (cur === 'volumes') volumeEnabled.value = true
  else if (cur === 'compose') composeEnabled.value = true
})
</script>

<style scoped>
.docker-tabs {
  background: var(--fp-bg-surface);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  padding: var(--fp-space-4);
}
</style>
