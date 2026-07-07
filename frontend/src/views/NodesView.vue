<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>节点管理</h2>
      <el-button type="primary" @click="showCreate = true">注册节点</el-button>
    </div>
    <el-card shadow="hover">
      <el-table :data="nodes" stripe v-loading="loading">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="name" label="名称" />
        <el-table-column prop="hostname" label="主机名" />
        <el-table-column prop="ip_address" label="IP 地址" />
        <el-table-column prop="status" label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.status === 'online' ? 'success' : 'info'" effect="plain" size="small">
              {{ row.status === 'online' ? '在线' : '离线' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180" />
      </el-table>
    </el-card>

    <el-dialog v-model="showCreate" title="注册节点" width="480px" destroy-on-close>
      <el-form :model="form" label-width="100px" :rules="rules" ref="formRef">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="主机名" prop="hostname">
          <el-input v-model="form.hostname" placeholder="例如 node1.example.com" />
        </el-form-item>
        <el-form-item label="IP 地址" prop="ip_address">
          <el-input v-model="form.ip_address" placeholder="例如 10.0.0.1" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="form.status" style="width:100%">
            <el-option label="在线" value="online" />
            <el-option label="离线" value="offline" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">取消</el-button>
        <el-button type="primary" @click="handleCreate" :loading="submitting">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { listNodes, createNode } from '@/api/nodes'
import type { ServerNode } from '@/types'

const nodes = ref<ServerNode[]>([])
const loading = ref(false)
const submitting = ref(false)
const showCreate = ref(false)
const formRef = ref()
const form = ref({ name: '', hostname: '', ip_address: '', status: 'online' })
const rules = {
  name: [{ required: true, message: '请输入节点名称', trigger: 'blur' }],
  hostname: [{ required: true, message: '请输入主机名', trigger: 'blur' }],
  ip_address: [{ required: true, message: '请输入 IP 地址', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listNodes()
    nodes.value = res.data
  } catch (e: any) {
    ElMessage.error('加载节点失败')
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    const node: any = { ...form.value }
    await createNode(node)
    ElMessage.success('节点注册成功')
    showCreate.value = false
    form.value = { name: '', hostname: '', ip_address: '', status: 'online' }
    await loadData()
  } catch (e: any) {
    ElMessage.error('注册失败')
  } finally {
    submitting.value = false
  }
}

onMounted(loadData)
</script>

<style scoped>
.card-header-title { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.card-header-title h2 { margin: 0; font-size: 18px; }
</style>
