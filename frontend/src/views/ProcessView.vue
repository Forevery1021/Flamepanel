<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import { ElMessage, ElTag } from 'element-plus'
import type { ProcessInfo } from '@/types'

const processes = ref<ProcessInfo[]>([])
const loading = ref(false)

const fetchProcesses = async () => {
  loading.value = true
  try {
    const { data } = await api.get<ProcessInfo[]>('/system/processes')
    processes.value = data
  } catch {
    ElMessage.error('获取进程列表失败')
  } finally {
    loading.value = false
  }
}

const statusType = (status: string) => {
  switch (status) {
    case 'Running': return 'success'
    case 'Sleeping': return 'info'
    case 'Stopped': return 'danger'
    case 'Zombie': return 'danger'
    default: return 'warning'
  }
}

onMounted(fetchProcesses)
</script>

<template>
  <div class="process-page">
    <div class="page-header">
      <h3>进程管理</h3>
      <el-button @click="fetchProcesses" :loading="loading">刷新</el-button>
    </div>

    <el-table :data="processes" v-loading="loading" stripe max-height="calc(100vh - 200px)">
      <el-table-column prop="pid" label="PID" width="80" />
      <el-table-column prop="name" label="进程名" min-width="200" />
      <el-table-column prop="cpu_usage" label="CPU %" width="120">
        <template #default="{ row }">{{ row.cpu_usage.toFixed(1) }}</template>
      </el-table-column>
      <el-table-column label="内存" width="120">
        <template #default="{ row }">{{ row.memory_mb.toFixed(1) }} MB</template>
      </el-table-column>
      <el-table-column label="状态" width="120">
        <template #default="{ row }">
          <ElTag :type="statusType(row.status)" size="small">{{ row.status }}</ElTag>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<style scoped>
.process-page {
  max-width: 1200px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-header h3 {
  margin: 0;
}
</style>
