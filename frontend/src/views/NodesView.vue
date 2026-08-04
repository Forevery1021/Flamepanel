<template>
  <LayoutContent :title="t('node.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('node.register') }}
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
        <Column field="id" :header="t('node.id')" style="width: 70px" />
        <Column field="name" :header="t('node.name')" />
        <Column :header="t('node.hostname')">
          <template #body="{ data }">
            <span v-tooltip="data.hostname" class="cell-truncate">{{ data.hostname }}</span>
          </template>
        </Column>
        <Column field="ip_address" :header="t('node.ip')" />
        <Column :header="t('node.status')" style="width: 110px">
          <template #body="{ data }">
            <FpTag
              :severity="data.online ? 'success' : 'danger'"
              :dot="data.online"
              :value="data.online ? t('dashboard.online') : t('dashboard.offline')"
            />
          </template>
        </Column>
        <Column :header="t('node.metrics')">
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
        </Column>
        <Column :header="t('node.lastHeartbeat')" style="width: 150px">
          <template #body="{ data }">
            <span v-if="data.last_heartbeat_at" class="text-xs">{{ data.last_heartbeat_at }}</span>
            <span v-else class="text-muted text-xs">—</span>
          </template>
        </Column>
        <Column :header="t('common.operation')" style="width: 130px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton variant="link" @click="confirmDelete(data)">{{ t('common.delete') }}</FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
      <Paginator
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total-records="total"
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
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
import { listNodes, createNode, updateNode, deleteNode, nodeStatus, nodeMetrics } from '@/api/nodes'
import type { ServerNode, NodeMetrics } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const nodes = ref<ServerNode[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const dialogVisible = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)

// 轮询句柄（页面隐藏时暂停）
let pollTimer: number | null = null
let lastVisible = document.visibilityState === 'visible'

const form = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const editForm = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const formErrors = reactive({ name: '', hostname: '', ip_address: '' })
const editErrors = reactive({ name: '', hostname: '', ip_address: '' })

const statusOptions = computed(() => [
  { label: t('dashboard.online'), value: 'online' },
  { label: t('dashboard.offline'), value: 'offline' },
])

function percentColor(p: number) {
  return p > 80 ? '#f56c6c' : p > 50 ? '#e6a23c' : '#67c23a'
}

function enrichNode(node: ServerNode) {
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
  // 逐节点查在线状态 + 指标（并发）
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
  nodes.value = updated
}

async function fetch() {
  loading.value = true
  try {
    const res = await listNodes(currentPage.value, pageSize.value)
    nodes.value = res.data.data.map(enrichNode)
    total.value = res.data.total
    await refreshStatus()
  } finally {
    loading.value = false
  }
}

function startPolling() {
  stopPolling()
  pollTimer = window.setInterval(() => {
    if (document.visibilityState === 'visible') {
      refreshStatus()
    }
  }, 10000)
}

function stopPolling() {
  if (pollTimer !== null) {
    clearInterval(pollTimer)
    pollTimer = null
  }
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

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
}

onMounted(() => {
  fetch()
  startPolling()
  document.addEventListener('visibilitychange', onVisibility)
})

onUnmounted(() => {
  stopPolling()
  document.removeEventListener('visibilitychange', onVisibility)
})
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
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
</style>
