<template>
  <div class="view-container">
    <div class="card-header-title">
      <el-button type="primary" @click="dialogVisible = true">{{ t('node.register') }}</el-button>
    </div>

    <el-card shadow="hover">
      <div class="toolbar">
        <span class="text-muted text-xs">{{ t('node.liveHint') }}</span>
      </div>
      <el-table v-loading="loading" :empty-text="t('common.noData')" :data="nodes" border stripe>
        <el-table-column prop="id" :label="t('node.id')" width="70" />
        <el-table-column prop="name" :label="t('node.name')" min-width="110" />
        <el-table-column prop="hostname" :label="t('node.hostname')" min-width="140" show-overflow-tooltip />
        <el-table-column prop="ip_address" :label="t('node.ip')" min-width="120" />
        <el-table-column :label="t('node.status')" width="110">
          <template #default="{ row }">
            <el-tag :type="row.online ? 'success' : 'danger'" size="small" effect="light">
              {{ row.online ? t('dashboard.online') : t('dashboard.offline') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('node.metrics')" min-width="240">
          <template #default="{ row }">
            <div v-if="row.metrics" class="metrics-row">
              <el-progress
                :percentage="Math.round(row.metrics.cpu_usage ?? 0)"
                :stroke-width="6"
                :color="percentColor(row.metrics.cpu_usage ?? 0)"
                class="metrics-bar"
              />
              <span class="text-xs text-muted"
                >CPU {{ (row.metrics.cpu_usage ?? 0).toFixed(1) }}% · MEM
                {{ (row.metrics.memory_usage_percent ?? 0).toFixed(1) }}% · DISK
                {{ (row.metrics.disk_usage_percent ?? 0).toFixed(1) }}%</span
              >
            </div>
            <span v-else class="text-muted text-xs">—</span>
          </template>
        </el-table-column>
        <el-table-column :label="t('node.lastHeartbeat')" width="150">
          <template #default="{ row }">
            <span v-if="row.last_heartbeat_at" class="text-xs">{{ row.last_heartbeat_at }}</span>
            <span v-else class="text-muted text-xs">—</span>
          </template>
        </el-table-column>
        <el-table-column :label="t('common.operation')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" size="small" text @click="handleEdit(row)">{{
              t('common.edit')
            }}</el-button>
            <el-popconfirm
              :title="t('node.deleteConfirm', { name: row.name })"
              @confirm="handleDelete(row.id)"
            >
              <template #reference>
                <el-button type="danger" size="small" text>{{ t('common.delete') }}</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        background
        small
        class="table-pagination"
        @current-change="fetch"
      />
    </el-card>

    <el-dialog v-model="dialogVisible" :title="t('node.register')" width="500px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item :label="t('node.name')" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('node.hostname')" prop="hostname">
          <el-input v-model="form.hostname" />
        </el-form-item>
        <el-form-item :label="t('node.ip')" prop="ip_address">
          <el-input v-model="form.ip_address" />
        </el-form-item>
        <el-form-item :label="t('node.status')" prop="status">
          <el-select v-model="form.status" class="full-width">
            <el-option :label="t('dashboard.online')" value="online" />
            <el-option :label="t('dashboard.offline')" value="offline" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreate">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="editVisible" :title="t('node.editNode')" width="500px">
      <el-form ref="editFormRef" :model="editForm" :rules="rules" label-width="100px">
        <el-form-item :label="t('node.name')" prop="name">
          <el-input v-model="editForm.name" />
        </el-form-item>
        <el-form-item :label="t('node.hostname')" prop="hostname">
          <el-input v-model="editForm.hostname" />
        </el-form-item>
        <el-form-item :label="t('node.ip')" prop="ip_address">
          <el-input v-model="editForm.ip_address" />
        </el-form-item>
        <el-form-item :label="t('node.status')" prop="status">
          <el-select v-model="editForm.status" class="full-width">
            <el-option :label="t('dashboard.online')" value="online" />
            <el-option :label="t('dashboard.offline')" value="offline" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSave">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listNodes, createNode, updateNode, deleteNode, nodeStatus, nodeMetrics } from '@/api/nodes'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { ServerNode, NodeMetrics } from '@/types'

const { t } = useI18n()
const nodes = ref<ServerNode[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const dialogVisible = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editFormRef = ref<FormInstance>()
const editingId = ref(0)

// 轮询句柄（页面隐藏时暂停）
let pollTimer: number | null = null
let lastVisible = document.visibilityState === 'visible'

const form = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const editForm = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const rules: FormRules = {
  name: [{ required: true, message: t('node.nameRequired'), trigger: 'blur' }],
  hostname: [{ required: true, message: t('node.hostnameRequired'), trigger: 'blur' }],
  ip_address: [{ required: true, message: t('node.ipRequired'), trigger: 'blur' }],
}

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

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
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
    ElMessage.success(t('common.success'))
    dialogVisible.value = false
    form.name = ''
    form.hostname = ''
    form.ip_address = ''
    form.status = 'online'
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
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
  editVisible.value = true
}

async function handleSave() {
  const valid = await editFormRef.value?.validate().catch(() => false)
  if (!valid) return
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
    ElMessage.success(t('common.success'))
    editVisible.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await deleteNode(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
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
.toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.metrics-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.metrics-bar {
  max-width: 200px;
}
</style>
