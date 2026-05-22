<script setup lang="ts">
import { onMounted, ref } from 'vue'
import api from '@/api/client'
import { ElTable, ElTableColumn, ElButton, ElTag, ElMessage } from 'element-plus'

interface DockerContainer {
  id: string
  name: string
  image: string
  status: string
  state: string
  ports: string
}

const containers = ref<DockerContainer[]>([])
const loading = ref(false)

const loadContainers = async () => {
  loading.value = true
  try {
    const res = await api.get('/docker/containers')
    containers.value = res.data
  } catch (e) {
    ElMessage.error('加载容器列表失败')
  } finally {
    loading.value = false
  }
}

const executeAction = async (id: string, action: string) => {
  try {
    await api.post('/docker/action', { id, action })
    ElMessage.success(`容器 ${action} 操作成功`)
    await loadContainers()
  } catch (e) {
    ElMessage.error(`操作失败`)
  }
}

onMounted(loadContainers)
</script>

<template>
  <ElCard>
    <template #header>
      <div class="flex justify-between">
        <span>Docker 容器管理</span>
        <ElButton type="primary" @click="loadContainers" :loading="loading">刷新</ElButton>
      </div>
    </template>

    <ElTable :data="containers" v-loading="loading" stripe>
      <ElTableColumn prop="name" label="容器名称" width="180" />
      <ElTableColumn prop="image" label="镜像" />
      <ElTableColumn prop="status" label="状态">
        <template #default="{ row }">
          <ElTag :type="row.state === 'running' ? 'success' : 'info'">
            {{ row.state }}
          </ElTag>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="ports" label="端口映射" />
      <ElTableColumn label="操作" width="220">
        <template #default="{ row }">
          <ElButton 
            v-if="row.state === 'running'" 
            size="small" 
            type="danger"
            @click="executeAction(row.id, 'stop')">
            停止
          </ElButton>
          <ElButton 
            v-else 
            size="small" 
            type="success"
            @click="executeAction(row.id, 'start')">
            启动
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>
  </ElCard>
</template>