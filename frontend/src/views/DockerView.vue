<script setup lang="ts">
import { onMounted, ref } from 'vue'
import api from '@/api/client'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElTag,
  ElMessage,
  ElCard,
  ElDialog,
  ElInput,
} from 'element-plus'
import type { DockerContainer, DockerImage } from '@/types'

const containers = ref<DockerContainer[]>([])
const images = ref<DockerImage[]>([])
const loading = ref(false)
const logsVisible = ref(false)
const logsContent = ref('')
const logsLoading = ref(false)
const currentContainer = ref<DockerContainer | null>(null)

const loadContainers = async () => {
  loading.value = true
  try {
    const res = await api.get<DockerContainer[]>('/docker/containers')
    containers.value = res.data
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载容器列表失败')
  } finally {
    loading.value = false
  }
}

const loadImages = async () => {
  try {
    const res = await api.get<DockerImage[]>('/docker/images')
    images.value = res.data
  } catch (e) {
    console.error('加载镜像列表失败', e)
  }
}

const executeAction = async (id: string, action: string) => {
  try {
    await api.post('/docker/containers/action', { id, action })
    ElMessage.success(`容器 ${action} 成功`)
    await loadContainers()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '操作失败')
  }
}

const viewLogs = async (container: DockerContainer) => {
  currentContainer.value = container
  logsVisible.value = true
  logsLoading.value = true
  try {
    const res = await api.get('/docker/containers/logs', {
      params: { id: container.id, tail: 200 },
    })
    logsContent.value = res.data.logs
  } catch (e: any) {
    ElMessage.error('加载日志失败')
    logsContent.value = ''
  } finally {
    logsLoading.value = false
  }
}

onMounted(() => {
  loadContainers()
  loadImages()
})
</script>

<template>
  <div class="docker-page">
    <!-- 容器列表 -->
    <ElCard>
      <template #header>
        <div style="display: flex; justify-content: space-between; align-items: center">
          <span>Docker 容器管理</span>
          <ElButton type="primary" @click="loadContainers" :loading="loading">刷新</ElButton>
        </div>
      </template>

      <ElTable :data="containers" v-loading="loading" stripe>
        <ElTableColumn prop="name" label="容器名称" width="200" />
        <ElTableColumn prop="image" label="镜像" min-width="180" />
        <ElTableColumn prop="state" label="状态" width="100">
          <template #default="{ row }">
            <ElTag :type="row.state === 'running' ? 'success' : 'info'">
              {{ row.state }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="status" label="运行状态" min-width="200" />
        <ElTableColumn label="端口映射" min-width="180">
          <template #default="{ row }">
            <ElTag
              v-for="port in row.ports"
              :key="port"
              size="small"
              style="margin-right: 4px"
            >
              {{ port }}
            </ElTag>
            <span v-if="!row.ports || row.ports.length === 0" style="color: #c0c4cc">-</span>
          </template>
        </ElTableColumn>
        <ElTableColumn label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <ElButton
              v-if="row.state === 'running'"
              size="small"
              type="danger"
              @click="executeAction(row.id, 'stop')"
            >
              停止
            </ElButton>
            <ElButton
              v-if="row.state !== 'running'"
              size="small"
              type="success"
              @click="executeAction(row.id, 'start')"
            >
              启动
            </ElButton>
            <ElButton
              size="small"
              @click="executeAction(row.id, 'restart')"
            >
              重启
            </ElButton>
            <ElButton size="small" type="info" @click="viewLogs(row)">日志</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 镜像列表 -->
    <ElCard style="margin-top: 20px">
      <template #header>
        <span>本地镜像</span>
      </template>
      <ElTable :data="images" stripe>
        <ElTableColumn prop="repository" label="仓库" />
        <ElTableColumn prop="tag" label="标签" width="120" />
        <ElTableColumn prop="id" label="镜像 ID" width="140" />
        <ElTableColumn prop="size" label="大小" width="120" />
        <ElTableColumn prop="created" label="创建时间" min-width="200" />
      </ElTable>
    </ElCard>

    <!-- 日志弹窗 -->
    <ElDialog
      v-model="logsVisible"
      :title="`容器日志 - ${currentContainer?.name || ''}`"
      width="800px"
      top="5vh"
    >
      <div
        v-loading="logsLoading"
        style="background: #1e1e1e; color: #d4d4d4; padding: 16px; border-radius: 4px; max-height: 500px; overflow: auto; font-family: monospace; font-size: 13px; white-space: pre-wrap;"
      >
        {{ logsContent || '无日志内容' }}
      </div>
    </ElDialog>
  </div>
</template>
