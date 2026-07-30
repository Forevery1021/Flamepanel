<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.nodes') }}</h2>
      <el-button type="primary" @click="dialogVisible = true">{{ t('node.register') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table :data="nodes" border stripe v-loading="loading">
        <el-table-column prop="id" :label="t('node.id')" width="80" />
        <el-table-column prop="name" :label="t('node.name')" />
        <el-table-column prop="hostname" :label="t('node.hostname')" />
        <el-table-column prop="ip_address" :label="t('node.ip')" />
        <el-table-column :label="t('node.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'online' ? 'success' : 'danger'" size="small">
              {{ row.status === 'online' ? t('dashboard.online') : t('dashboard.offline') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('node.createdAt')" width="180" />
        <el-table-column :label="t('common.operation')" width="120" fixed="right">
          <template #default="{ row }">
            <el-popconfirm :title="t('node.deleteConfirm', { name: row.name })" @confirm="handleDelete(row.id)">
              <template #reference>
                <el-button type="danger" size="small" text>{{ t('common.delete') }}</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="t('node.register')" width="500px">
      <el-form :model="form" ref="formRef" :rules="rules" label-width="100px">
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
          <el-select v-model="form.status" style="width:100%">
            <el-option :label="t('dashboard.online')" value="online" />
            <el-option :label="t('dashboard.offline')" value="offline" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="handleCreate" :loading="submitting">{{ t('common.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listNodes, createNode, deleteNode } from '@/api/nodes'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { ServerNode } from '@/types'

const { t } = useI18n()
const nodes = ref<ServerNode[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({ name: '', hostname: '', ip_address: '', status: 'online' })
const rules: FormRules = {
  name: [{ required: true, message: '请输入节点名称', trigger: 'blur' }],
  hostname: [{ required: true, message: '请输入主机名', trigger: 'blur' }],
  ip_address: [{ required: true, message: '请输入 IP 地址', trigger: 'blur' }],
}

async function fetch() {
  loading.value = true
  try {
    const res = await listNodes()
    nodes.value = res.data
  } finally { loading.value = false }
}

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    await createNode({ id: 0, name: form.name, hostname: form.hostname, ip_address: form.ip_address, status: form.status, created_at: '' })
    ElMessage.success(t('common.success'))
    dialogVisible.value = false
    form.name = ''; form.hostname = ''; form.ip_address = ''; form.status = 'online'
    await fetch()
  } catch { ElMessage.error(t('common.failed')) }
  finally { submitting.value = false }
}

async function handleDelete(id: number) {
  try {
    await deleteNode(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch { ElMessage.error(t('common.failed')) }
}

onMounted(fetch)
</script>
