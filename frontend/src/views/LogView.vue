<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import { ElMessage } from 'element-plus'
import type { OperationLogEntry } from '@/types'

const logs = ref<OperationLogEntry[]>([])
const loading = ref(false)
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)

const fetchLogs = async () => {
  loading.value = true
  try {
    const { data } = await api.get('/logs/list', {
      params: { page: page.value, page_size: pageSize.value }
    })
    logs.value = data.items
    total.value = data.total
  } catch {
    ElMessage.error('获取日志失败')
  } finally {
    loading.value = false
  }
}

const handlePageChange = (p: number) => {
  page.value = p
  fetchLogs()
}

onMounted(fetchLogs)
</script>

<template>
  <div class="log-page">
    <div class="page-header">
      <h3>操作日志</h3>
      <span class="total">共 {{ total }} 条</span>
    </div>

    <el-table :data="logs" v-loading="loading" stripe>
      <el-table-column prop="username" label="用户" width="120" />
      <el-table-column prop="action" label="操作" width="160" />
      <el-table-column prop="target" label="目标" min-width="200" />
      <el-table-column prop="ip" label="IP" width="150" />
      <el-table-column prop="created_at" label="时间" width="180" />
    </el-table>

    <div class="pagination" v-if="total > pageSize">
      <el-pagination
        background
        layout="prev, pager, next"
        :total="total"
        :page-size="pageSize"
        :current-page="page"
        @current-change="handlePageChange"
      />
    </div>
  </div>
</template>

<style scoped>
.log-page {
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
.total {
  color: #909399;
  font-size: 14px;
}
.pagination {
  margin-top: 20px;
  display: flex;
  justify-content: center;
}
</style>
