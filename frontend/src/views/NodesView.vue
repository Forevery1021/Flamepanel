<template>
  <LayoutContent :title="t('node.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton v-permission="{ perm: 'node:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('node.register') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="ghost" @click="openExecModal">
        {{ t('node.remoteExec') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="ghost" @click="openBatchModal">
        {{ t('node.batchExec') }}
      </FpButton>
    </template>

    <div class="panel">
      <div class="toolbar">
        <span class="text-muted text-xs">{{ t('node.liveHint') }}</span>
      </div>
      <FpTable
        :rows="nodes"
        :loading="loading"
        :first="(currentPage - 1) * pageSize"
        :empty-text="t('common.noData')"
      >
        <FpColumn field="id" :header="t('node.id')" style="width: 70px" />
        <FpColumn field="name" :header="t('node.name')" />
        <FpColumn :header="t('node.hostname')">
          <template #body="{ data }">
            <span v-tooltip="data.hostname" class="cell-truncate">{{ data.hostname }}</span>
          </template>
        </FpColumn>
        <FpColumn field="ip_address" :header="t('node.ip')" />
        <FpColumn :header="t('node.status')" style="width: 110px">
          <template #body="{ data }">
            <FpTag
              :severity="data.online ? 'success' : 'danger'"
              :dot="data.online"
              :value="data.online ? t('dashboard.online') : t('dashboard.offline')"
            />
          </template>
        </FpColumn>
        <FpColumn :header="t('node.metrics')">
          <template #body="{ data }">
            <div v-if="data.metrics" class="metrics-row">
              <div class="metrics-bar">
                <div
                  class="metrics-bar-fill"
                  :style="{
                    width: `${Math.round(data.metrics.cpu_usage ?? 0)}%`,
                    background: percentColor(data.metrics.cpu_usage ?? 0),
                  }"
                />
              </div>
              <span class="text-xs text-muted"
                >CPU {{ (data.metrics.cpu_usage ?? 0).toFixed(1) }}% · MEM
                {{ (data.metrics.memory_usage_percent ?? 0).toFixed(1) }}% · DISK
                {{ (data.metrics.disk_usage_percent ?? 0).toFixed(1) }}%</span
              >
            </div>
            <span v-else class="text-muted text-xs">—</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('node.lastHeartbeat')" style="width: 150px">
          <template #body="{ data }">
            <span v-if="data.last_heartbeat_at" class="text-xs">{{ data.last_heartbeat_at }}</span>
            <span v-else class="text-muted text-xs">—</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('common.operation')" style="width: 130px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton v-permission="{ perm: 'node:update', mode: 'view' }" variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="link" @click="openExecForNode(data)">{{ t('node.remoteExec') }}</FpButton>
              <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="link" @click="openFilesForNode(data)">{{ t('node.remoteFiles') }}</FpButton>
              <FpButton v-permission="{ perm: 'node:delete', mode: 'view' }" variant="link" @click="confirmDelete(data)">{{ t('common.delete') }}</FpButton>
            </div>
          </template>
        </FpColumn>
      </FpTable>
      <FpPagination
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => goPage(f)"
      />
    </div>

    <FpModal v-model="dialogVisible" :header="t('node.register')" style="width: 500px">
      <div class="modal-form">
        <FpInput v-model="form.name" :label="t('node.name')" :error="formErrors.name" />
        <FpInput v-model="form.hostname" :label="t('node.hostname')" :error="formErrors.hostname" />
        <FpInput v-model="form.ip_address" :label="t('node.ip')" :error="formErrors.ip_address" />
        <FpSelect
          v-model="form.status"
          :label="t('node.status')"
          :options="statusOptions"
          option-label="label"
          option-value="value"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleCreate">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="editVisible" :header="t('node.editNode')" style="width: 500px">
      <div class="modal-form">
        <FpInput v-model="editForm.name" :label="t('node.name')" :error="editErrors.name" />
        <FpInput
          v-model="editForm.hostname"
          :label="t('node.hostname')"
          :error="editErrors.hostname"
        />
        <FpInput
          v-model="editForm.ip_address"
          :label="t('node.ip')"
          :error="editErrors.ip_address"
        />
        <FpSelect
          v-model="editForm.status"
          :label="t('node.status')"
          :options="statusOptions"
          option-label="label"
          option-value="value"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="editVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSave">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <!-- Stage5：远程命令 -->
    <FpModal v-model="execVisible" :header="t('node.remoteExec')" style="width: 640px">
      <div class="modal-form">
        <div v-if="execNodeName" class="text-muted text-xs">
          {{ t('node.name') }}: {{ execNodeName }}
        </div>
        <FpInput
          v-model="execCommand"
          :label="t('node.command')"
          :placeholder="t('node.commandPlaceholder')"
          :error="execErrors.command"
        />
        <div v-if="execResult" class="exec-result">
          <div class="exec-result-head">
            <span class="text-muted text-xs">
              {{ t('node.exitCode') }}: {{ execResult.exit_code }} ·
              {{ t('node.duration') }}: {{ execResult.duration_ms }}ms
            </span>
          </div>
          <pre class="exec-output">{{ execResult.output || '—' }}</pre>
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="execVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="execLoading" @click="handleExec">
          {{ t('node.run') }}
        </FpButton>
      </template>
    </FpModal>

    <!-- Stage5：批量命令 -->
    <FpModal v-model="batchVisible" :header="t('node.batchExec')" style="width: 680px">
      <div class="modal-form">
        <div class="text-muted text-xs">{{ t('node.selectNodesHint') }}</div>
        <div class="batch-node-list">
          <label v-for="n in nodes" :key="n.id" class="batch-node-item">
            <input
              v-model="batchSelectedIds"
              type="checkbox"
              :value="n.id"
            />
            <span>{{ n.name }} ({{ n.ip_address }})</span>
          </label>
        </div>
        <FpInput
          v-model="batchCommand"
          :label="t('node.command')"
          :placeholder="t('node.commandPlaceholder')"
        />
        <div v-if="batchResult.length" class="batch-result">
          <div v-for="item in batchResult" :key="item.node_id" class="batch-result-item">
            <div class="batch-result-head">
              <span class="text-sm">[{{ item.node_name }}]</span>
              <FpTag
                :severity="item.success ? 'success' : 'danger'"
                :value="item.success ? 'OK' : 'ERR'"
              />
            </div>
            <pre class="exec-output">{{ item.result?.output || item.result?.error || '—' }}</pre>
          </div>
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="batchVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="batchLoading" @click="handleBatch">
          {{ t('node.runBatch') }}
        </FpButton>
      </template>
    </FpModal>

    <!-- Stage5：远程文件 -->
    <FpModal v-model="filesVisible" :header="t('node.nodeFiles')" style="width: 720px">
      <div class="modal-form">
        <div v-if="filesNodeName" class="text-muted text-xs">
          {{ t('node.name') }}: {{ filesNodeName }}
        </div>
        <div class="files-path-bar">
          <FpButton variant="ghost" size="small" @click="filesGoUp">{{ t('node.up') }}</FpButton>
          <FpInput
            v-model="filesPath"
            :placeholder="t('node.pathPlaceholder')"
            class="flex-1"
          />
          <FpButton variant="primary" size="small" @click="filesLoad">{{ t('node.go') }}</FpButton>
          <FpButton variant="ghost" size="small" @click="filesLoad">{{ t('node.refresh') }}</FpButton>
        </div>
        <div v-if="filesEntries.length" class="files-list">
          <div
            v-for="e in filesEntries"
            :key="e.name"
            class="files-row"
            @click="e.is_dir && filesEnter(e.name)"
          >
            <span class="files-icon">{{ e.is_dir ? '📁' : '📄' }}</span>
            <span class="files-name cell-truncate">{{ e.name }}</span>
            <span class="text-muted text-xs">{{ e.is_dir ? '—' : formatBytes(e.size) }}</span>
            <div class="files-actions">
              <FpButton
                v-if="!e.is_dir"
                variant="link"
                size="small"
                @click.stop="filesDownload(e.name)"
              >
                {{ t('node.download') }}
              </FpButton>
            </div>
          </div>
          <div v-if="!filesEntries.length" class="text-muted text-xs">—</div>
        </div>
        <div v-else class="text-muted text-xs">—</div>
        <FpInput v-model="filesUploadContent" :label="t('node.uploadFile')" />
        <div class="files-upload-bar">
          <FpInput v-model="filesUploadName" placeholder="filename.txt" class="flex-1" />
          <FpButton variant="primary" size="small" :loading="filesUploading" @click="filesUpload">
            {{ t('node.upload') }}
          </FpButton>
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="filesVisible = false">{{ t('common.close') || t('common.cancel') }}</FpButton>
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'


import { listNodes, createNode, updateNode, deleteNode, nodeStatus, nodeMetrics, remoteExecute, batchExecute, remoteListFiles, remoteDownloadFile, remoteUploadFile } from '@/api/nodes'
import type { ServerNode, NodeMetrics, RemoteFileEntry, RemoteExecResult, BatchExecItem } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getErrorMessage } from '@/utils/error'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()

const currentPage = ref(1)
const pageSize = ref(20)
const dialogVisible = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)

// 页面隐藏时暂停状态轮询
let lastVisible = document.visibilityState === 'visible'

const form = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const editForm = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const formErrors = reactive({ name: '', hostname: '', ip_address: '' })
const editErrors = reactive({ name: '', hostname: '', ip_address: '' })

// ── Stage5 远程命令 / 批量 / 文件 ──
const execVisible = ref(false)
const execNodeId = ref(0)
const execNodeName = ref('')
const execCommand = ref('')
const execErrors = reactive({ command: '' })
const execLoading = ref(false)
const execResult = ref<RemoteExecResult | null>(null)

const batchVisible = ref(false)
const batchCommand = ref('')
const batchSelectedIds = ref<number[]>([])
const batchLoading = ref(false)
const batchResult = ref<BatchExecItem[]>([])

const filesVisible = ref(false)
const filesNodeId = ref(0)
const filesNodeName = ref('')
const filesPath = ref('.')
const filesEntries = ref<RemoteFileEntry[]>([])
const filesUploading = ref(false)
const filesUploadName = ref('')
const filesUploadContent = ref('')

const statusOptions = computed(() => [
  { label: t('dashboard.online'), value: 'online' },
  { label: t('dashboard.offline'), value: 'offline' },
])

/** 节点 + 实时指标/在线状态（F1.1 缓存写入） */
interface EnrichedNode extends ServerNode {
  metrics: NodeMetrics | null
  online: boolean
}

function percentColor(p: number) {
  return p > 80
    ? 'var(--fp-danger)'
    : p > 50
      ? 'var(--fp-warning)'
      : 'var(--fp-success)'
}

function enrichNode(node: ServerNode): EnrichedNode {
  let metrics: NodeMetrics | null = null
  if (node.metrics_json) {
    try {
      metrics = JSON.parse(node.metrics_json)
    } catch {
      metrics = null
    }
  }
  return { ...node, metrics, online: false }
}

async function refreshStatus() {
  if (!nodes.value.length) return
  // 逐节点查在线状态 + 指标（并发），结果写入查询缓存（WS/列表双源合一）
  const updated = await Promise.all(
    nodes.value.map(async (node) => {
      try {
        const [st, mt] = await Promise.all([nodeStatus(node.id), nodeMetrics(node.id)])
        return {
          ...node,
          online: st.data.status === 'online',
          metrics: mt.data as NodeMetrics,
        }
      } catch {
        return { ...node, online: false }
      }
    }),
  )
  queryClient.setQueryData(queryKeys.nodes.list(currentPage.value, pageSize.value), (old: { data: ServerNode[]; total: number } | undefined) => {
    if (!old) return old
    return { data: updated, total: old.total }
  })
}

// F1.1：节点列表走统一数据获取层；状态通过轮询写入缓存，页签/页面不可见时自动暂停
const nodesQuery = useApiQuery<{ data: EnrichedNode[]; total: number }>(
  () => queryKeys.nodes.list(currentPage.value, pageSize.value),
  async () => {
    const res = await listNodes(currentPage.value, pageSize.value)
    const enriched = res.data.data.map(enrichNode)
    return { data: { data: enriched, total: res.data.total } }
  },
  {
    keepPrevious: true,
    refetchInterval: 10_000,
    refetchIntervalInBackground: false,
  },
)
const nodes = computed<EnrichedNode[]>(() => nodesQuery.data.value?.data ?? [])
const loading = nodesQuery.loading
const total = computed(() => nodesQuery.data.value?.total ?? 0)

async function fetch() {
  await nodesQuery.refresh()
  await refreshStatus()
}

function onVisibility() {
  const visible = document.visibilityState === 'visible'
  if (visible && !lastVisible) {
    refreshStatus()
  }
  lastVisible = visible
}

function openCreate() {
  form.name = ''
  form.hostname = ''
  form.ip_address = ''
  form.status = 'online'
  formErrors.name = ''
  formErrors.hostname = ''
  formErrors.ip_address = ''
  dialogVisible.value = true
}

function validateForm(): boolean {
  formErrors.name = form.name ? '' : t('node.nameRequired')
  formErrors.hostname = form.hostname ? '' : t('node.hostnameRequired')
  formErrors.ip_address = form.ip_address ? '' : t('node.ipRequired')
  return !formErrors.name && !formErrors.hostname && !formErrors.ip_address
}

async function handleCreate() {
  if (!validateForm()) return
  submitting.value = true
  try {
    await createNode({
      id: 0,
      name: form.name,
      hostname: form.hostname,
      ip_address: form.ip_address,
      status: form.status,
      created_at: '',
    })
    toast.success(t('common.success'))
    dialogVisible.value = false
    form.name = ''
    form.hostname = ''
    form.ip_address = ''
    form.status = 'online'
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function handleEdit(row: ServerNode) {
  editingId.value = row.id
  editForm.name = row.name
  editForm.hostname = row.hostname
  editForm.ip_address = row.ip_address
  editForm.status = row.status
  editErrors.name = ''
  editErrors.hostname = ''
  editErrors.ip_address = ''
  editVisible.value = true
}

function validateEditForm(): boolean {
  editErrors.name = editForm.name ? '' : t('node.nameRequired')
  editErrors.hostname = editForm.hostname ? '' : t('node.hostnameRequired')
  editErrors.ip_address = editForm.ip_address ? '' : t('node.ipRequired')
  return !editErrors.name && !editErrors.hostname && !editErrors.ip_address
}

async function handleSave() {
  if (!validateEditForm()) return
  submitting.value = true
  try {
    await updateNode(editingId.value, {
      id: editingId.value,
      name: editForm.name,
      hostname: editForm.hostname,
      ip_address: editForm.ip_address,
      status: editForm.status,
      created_at: '',
    })
    toast.success(t('common.success'))
    editVisible.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await deleteNode(id)
    toast.success(t('common.success'))
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}

function confirmDelete(row: ServerNode) {
  confirmAction({
    message: t('node.deleteConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: () => handleDelete(row.id),
  })
}

// ── Stage5 远程命令 ──
function openExecModal() {
  execNodeId.value = 0
  execNodeName.value = ''
  execCommand.value = ''
  execErrors.command = ''
  execResult.value = null
  execVisible.value = true
}

function openExecForNode(row: ServerNode) {
  execNodeId.value = row.id
  execNodeName.value = row.name
  execCommand.value = ''
  execErrors.command = ''
  execResult.value = null
  execVisible.value = true
}

async function handleExec() {
  if (!execCommand.value.trim()) {
    execErrors.command = t('node.commandRequired')
    return
  }
  if (execNodeId.value <= 0) {
    toast.error(t('node.nodeNotSelected'))
    return
  }
  execLoading.value = true
  try {
    const res = await remoteExecute(execNodeId.value, execCommand.value.trim(), 30)
    execResult.value = res.data
  } catch (e: unknown) {
    execResult.value = {
      output: getErrorMessage(e, t('node.agentUnreachable')),
      exit_code: -1,
      duration_ms: 0,
    }
  } finally {
    execLoading.value = false
  }
}

// ── Stage5 批量命令 ──
function openBatchModal() {
  batchCommand.value = ''
  batchSelectedIds.value = nodes.value.map((n) => n.id)
  batchResult.value = []
  batchVisible.value = true
}

async function handleBatch() {
  if (!batchCommand.value.trim()) {
    toast.error(t('node.commandRequired'))
    return
  }
  if (!batchSelectedIds.value.length) {
    toast.error(t('node.nodeNotSelected'))
    return
  }
  batchLoading.value = true
  try {
    const res = await batchExecute(batchSelectedIds.value, batchCommand.value.trim(), 30)
    batchResult.value = res.data.items
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('common.failed')))
  } finally {
    batchLoading.value = false
  }
}

// ── Stage5 远程文件 ──
function openFilesForNode(row: ServerNode) {
  filesNodeId.value = row.id
  filesNodeName.value = row.name
  filesPath.value = '.'
  filesEntries.value = []
  filesUploadName.value = ''
  filesUploadContent.value = ''
  filesVisible.value = true
  filesLoad()
}

async function filesLoad() {
  if (filesNodeId.value <= 0) return
  try {
    const res = await remoteListFiles(filesNodeId.value, filesPath.value || '.')
    filesEntries.value = res.data
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('node.agentUnreachable')))
    filesEntries.value = []
  }
}

function filesEnter(name: string) {
  filesPath.value = filesPath.value === '.' ? name : `${filesPath.value}/${name}`
  filesLoad()
}

function filesGoUp() {
  const parts = filesPath.value.split('/').filter(Boolean)
  parts.pop()
  filesPath.value = parts.length ? parts.join('/') : '.'
  filesLoad()
}

function formatBytes(n: number) {
  if (!n) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  let v = n
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

async function filesDownload(name: string) {
  const full = filesPath.value === '.' ? name : `${filesPath.value}/${name}`
  try {
    const res = await remoteDownloadFile(filesNodeId.value, full)
    const bytes = Uint8Array.from(atob(res.data.content_base64), (c) => c.charCodeAt(0))
    const blob = new Blob([bytes])
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = name
    a.click()
    URL.revokeObjectURL(url)
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('node.agentUnreachable')))
  }
}

async function filesUpload() {
  if (!filesUploadName.value.trim() || !filesUploadContent.value) {
    toast.error(t('node.fileContent'))
    return
  }
  filesUploading.value = true
  try {
    const full = filesPath.value === '.' ? filesUploadName.value.trim() : `${filesPath.value}/${filesUploadName.value.trim()}`
    const contentBase64 = btoa(unescape(encodeURIComponent(filesUploadContent.value)))
    await remoteUploadFile(filesNodeId.value, full, contentBase64)
    toast.success(t('common.success'))
    filesUploadName.value = ''
    filesUploadContent.value = ''
    await filesLoad()
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('common.failed')))
  } finally {
    filesUploading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
}

onMounted(() => {
  fetch()
  document.addEventListener('visibilitychange', onVisibility)
})

onUnmounted(() => {
  document.removeEventListener('visibilitychange', onVisibility)
})
</script>

<style scoped>
.toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.cell-truncate {
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.metrics-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.metrics-bar {
  width: 200px;
  height: 6px;
  border-radius: 3px;
  background: var(--fp-bg-hover);
  overflow: hidden;
}
.metrics-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 200ms var(--fp-ease-out);
}
/* Stage5 远程命令/批量/文件 */
.exec-result {
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-3);
  background: var(--fp-bg-subtle);
}
.exec-result-head {
  margin-bottom: 6px;
}
.exec-output {
  margin: 0;
  padding: var(--fp-space-3);
  background: var(--fp-bg-hover);
  border-radius: var(--fp-radius-sm);
  font-family: var(--fp-font-mono, monospace);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 240px;
  overflow-y: auto;
}
.batch-node-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-3);
}
.batch-node-item {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 13px;
}
.batch-result {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}
.batch-result-item {
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  padding: var(--fp-space-2);
}
.batch-result-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}
.files-path-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.files-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  max-height: 300px;
  overflow-y: auto;
}
.files-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--fp-border);
  cursor: default;
}
.files-row:hover {
  background: var(--fp-bg-hover);
}
.files-icon {
  width: 20px;
  text-align: center;
}
.files-name {
  flex: 1;
}
.files-actions {
  display: flex;
  gap: 4px;
}
.files-upload-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.flex-1 {
  flex: 1;
}
</style>
