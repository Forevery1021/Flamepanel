<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import { ElMessage, ElButton, ElInput } from 'element-plus'

interface FileItem {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: string
}

const files = ref<FileItem[]>([])
const currentPath = ref('/www')
const loading = ref(false)
const newFolderName = ref('')

const loadFiles = async () => {
  loading.value = true
  try {
    const res = await api.get('/file/list', { 
      params: { path: currentPath.value } 
    })
    files.value = res.data
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const createFolder = async () => {
  if (!newFolderName.value) return
  try {
    await api.post('/file/mkdir', `${currentPath.value}/${newFolderName.value}`)
    ElMessage.success('文件夹创建成功')
    newFolderName.value = ''
    loadFiles()
  } catch (e) {
    ElMessage.error('创建失败')
  }
}

onMounted(loadFiles)
</script>

<template>
  <ElCard>
    <template #header>
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <span class="font-medium">文件管理</span>
          <ElInput v-model="currentPath" size="small" style="width: 380px" @keyup.enter="loadFiles" />
        </div>
        <div class="flex gap-2">
          <ElInput v-model="newFolderName" placeholder="新建文件夹" size="small" style="width: 160px" />
          <ElButton size="small" type="primary" @click="createFolder">新建</ElButton>
          <ElButton size="small" @click="loadFiles">刷新</ElButton>
        </div>
      </div>
    </template>

    <ElTable :data="files" v-loading="loading" stripe height="calc(100vh - 280px)">
      <ElTableColumn prop="name" label="文件名" />
      <ElTableColumn prop="size" label="大小" width="120">
        <template #default="{ row }">
          {{ row.is_dir ? '-' : (row.size / 1024).toFixed(1) + ' KB' }}
        </template>
      </ElTableColumn>
      <ElTableColumn prop="modified" label="修改时间" />
      <ElTableColumn label="操作" width="180">
        <template #default="{ row }">
          <ElButton v-if="!row.is_dir" size="small" @click="() => {}">查看内容</ElButton>
          <ElButton size="small" type="danger">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>
  </ElCard>
</template>