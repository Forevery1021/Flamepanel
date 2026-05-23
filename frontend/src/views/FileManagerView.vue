<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import api from '@/api/client'
import {
  ElMessage,
  ElMessageBox,
  ElButton,
  ElInput,
  ElCard,
  ElTable,
  ElTableColumn,
  ElDialog,
  ElUpload,
  ElIcon,
} from 'element-plus'
import { Upload, FolderAdd, Download, Edit } from '@element-plus/icons-vue'
import type { FileItem } from '@/types'

const files = ref<FileItem[]>([])
const currentPath = ref('/www')
const loading = ref(false)
const newFolderName = ref('')
const editorVisible = ref(false)
const editorContent = ref('')
const editingFile = ref<FileItem | null>(null)
const uploadVisible = ref(false)
const uploadUrl = ref('')
const renameVisible = ref(false)
const renameTarget = ref<FileItem | null>(null)
const renameName = ref('')

const uploadHeaders = computed(() => ({
  Authorization: `Bearer ${localStorage.getItem('token') || ''}`,
}))

const loadFiles = async () => {
  loading.value = true
  try {
    const res = await api.get<FileItem[]>('/file/list', {
      params: { path: currentPath.value },
    })
    files.value = res.data
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const navigateTo = (file: FileItem) => {
  if (file.is_dir) {
    currentPath.value = file.path
    loadFiles()
  }
}

const goUp = () => {
  const parent = currentPath.value.split('/').slice(0, -1).join('/') || '/'
  currentPath.value = parent
  loadFiles()
}

const createFolder = async () => {
  if (!newFolderName.value) return
  try {
    await api.post('/file/mkdir', {
      path: `${currentPath.value}/${newFolderName.value}`,
    })
    ElMessage.success('文件夹创建成功')
    newFolderName.value = ''
    loadFiles()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '创建失败')
  }
}

const viewFile = async (file: FileItem) => {
  try {
    const res = await api.get<string>('/file/read', {
      params: { path: file.path },
    })
    editingFile.value = file
    editorContent.value = typeof res.data === 'string' ? res.data : JSON.stringify(res.data)
    editorVisible.value = true
  } catch (e: any) {
    ElMessage.error('读取文件失败')
  }
}

const saveFile = async () => {
  if (!editingFile.value) return
  try {
    await api.post('/file/write', {
      path: editingFile.value.path,
      content: editorContent.value,
    })
    ElMessage.success('保存成功')
    editorVisible.value = false
  } catch (e: any) {
    ElMessage.error('保存失败')
  }
}

const deleteItem = async (file: FileItem) => {
  try {
    await ElMessageBox.confirm(`确定要删除 ${file.name} 吗？`, '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await api.delete('/file/delete', { params: { path: file.path } })
    ElMessage.success('删除成功')
    loadFiles()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.response?.data?.message || '删除失败')
    }
  }
}

const downloadFile = (file: FileItem) => {
  const token = localStorage.getItem('token') || ''
  const url = `/api/file/download?path=${encodeURIComponent(file.path)}&token=${encodeURIComponent(token)}`
  window.open(url, '_blank')
}

const openRename = (file: FileItem) => {
  renameTarget.value = file
  renameName.value = file.name
  renameVisible.value = true
}

const doRename = async () => {
  if (!renameTarget.value || !renameName.value) return
  const dir = renameTarget.value.path.substring(0, renameTarget.value.path.lastIndexOf('/')) || '/'
  const newPath = dir + '/' + renameName.value
  try {
    await api.post('/file/rename', { old_path: renameTarget.value.path, new_path: newPath })
    ElMessage.success('重命名成功')
    renameVisible.value = false
    loadFiles()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '重命名失败')
  }
}

const startUpload = () => {
  uploadUrl.value = `/api/file/upload?dir=${encodeURIComponent(currentPath.value)}`
  uploadVisible.value = true
}

const onUploadSuccess = () => {
  ElMessage.success('上传成功')
  uploadVisible.value = false
  loadFiles()
}

onMounted(loadFiles)
</script>

<template>
  <div class="file-page">
    <ElCard>
      <template #header>
        <div style="display: flex; align-items: center; justify-content: space-between">
          <div style="display: flex; align-items: center; gap: 12px">
            <span style="font-weight: 500">文件管理</span>
            <ElInput
              v-model="currentPath"
              size="small"
              style="width: 400px"
              @keyup.enter="loadFiles"
            />
            <ElButton size="small" @click="goUp">上级目录</ElButton>
          </div>
          <div style="display: flex; gap: 8px">
            <ElInput
              v-model="newFolderName"
              placeholder="新建文件夹"
              size="small"
              style="width: 160px"
              @keyup.enter="createFolder"
            />
            <ElButton size="small" type="primary" @click="createFolder">
              <ElIcon><FolderAdd /></ElIcon>
              新建
            </ElButton>
            <ElButton size="small" type="success" @click="startUpload">
              <ElIcon><Upload /></ElIcon>
              上传
            </ElButton>
            <ElButton size="small" @click="loadFiles">刷新</ElButton>
          </div>
        </div>
      </template>

      <ElTable :data="files" v-loading="loading" stripe height="calc(100vh - 280px)">
        <ElTableColumn prop="name" label="文件名" min-width="200">
          <template #default="{ row }">
            <span
              :style="{ cursor: row.is_dir ? 'pointer' : 'default', color: row.is_dir ? '#409eff' : 'inherit' }"
              @click="navigateTo(row)"
            >
              {{ row.is_dir ? '📁' : '📄' }} {{ row.name }}
            </span>
          </template>
        </ElTableColumn>
        <ElTableColumn label="大小" width="120">
          <template #default="{ row }">
            {{ row.is_dir ? '-' : (row.size / 1024).toFixed(1) + ' KB' }}
          </template>
        </ElTableColumn>
        <ElTableColumn prop="modified" label="修改时间" width="180" />
        <ElTableColumn prop="permissions" label="权限" width="100" />
        <ElTableColumn label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <ElButton v-if="!row.is_dir" size="small" type="primary" :icon="Edit" @click="viewFile(row)">查看</ElButton>
            <ElButton v-if="!row.is_dir" size="small" :icon="Download" @click="downloadFile(row)">下载</ElButton>
            <ElButton size="small" @click="openRename(row)">重命名</ElButton>
            <ElButton size="small" type="danger" @click="deleteItem(row)">删除</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 文件编辑弹窗 -->
    <ElDialog
      v-model="editorVisible"
      :title="`编辑 - ${editingFile?.name || ''}`"
      width="800px"
      top="5vh"
    >
      <ElInput
        v-model="editorContent"
        type="textarea"
        :rows="20"
        style="font-family: monospace"
      />
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveFile">保存</ElButton>
      </template>
    </ElDialog>

    <!-- 重命名弹窗 -->
    <ElDialog v-model="renameVisible" title="重命名" width="420px">
      <ElInput v-model="renameName" placeholder="新文件名" @keyup.enter="doRename" />
      <template #footer>
        <ElButton @click="renameVisible = false">取消</ElButton>
        <ElButton type="primary" @click="doRename">确认</ElButton>
      </template>
    </ElDialog>

    <!-- 文件上传弹窗 -->
    <ElDialog v-model="uploadVisible" title="上传文件" width="500px">
      <ElUpload
        :action="uploadUrl"
        :headers="uploadHeaders"
        :on-success="onUploadSuccess"
        multiple
        drag
      >
        <div style="padding: 20px 0">
          <ElIcon :size="48" color="#409eff"><Upload /></ElIcon>
          <p>拖拽文件到此处或点击上传</p>
        </div>
      </ElUpload>
    </ElDialog>
  </div>
</template>
