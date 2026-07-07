<template>
  <div>
    <h2>Operation Logs</h2>
    <div style="margin-top:16px;font-size:13px;color:#909399;margin-bottom:8px">
      Audit trail of all user actions
    </div>
    <el-table :data="logs" border stripe v-loading="loading" max-height="620px">
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="username" label="User" width="120" />
      <el-table-column prop="action" label="Action" width="150">
        <template #default="{ row }">
          <el-tag size="small">{{ row.action }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="target" label="Target" min-width="200" />
      <el-table-column prop="ip" label="IP" width="140" />
      <el-table-column prop="created_at" label="Time" width="180" />
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { listOperationLogs } from '@/api/logs'
import { ElMessage } from 'element-plus'
import type { OperationLog } from '@/types'

const logs = ref<OperationLog[]>([])
const loading = ref(false)

async function fetch() {
  loading.value = true
  try { logs.value = (await listOperationLogs()).data } catch { ElMessage.error('获取审计日志失败') } finally { loading.value = false }
}

onMounted(fetch)
</script>
