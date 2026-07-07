<template>
  <div class="files-view">
    <div class="header">
      <h2>File Manager</h2>
      <div class="actions">
        <el-button @click="showUpload = true">Upload</el-button>
        <el-button @click="showCreateFile = true">New File</el-button>
        <el-button @click="showCreateDir = true">New Folder</el-button>
        <el-button @click="fetch" :loading="loading">Refresh</el-button>
      </div>
    </div>

    <el-breadcrumb separator="/" style="margin: 12px 0">
      <el-breadcrumb-item v-for="(seg, i) in breadcrumbs" :key="i">
        <a @click="goTo(seg.path)">{{ seg.name }}</a>
      </el-breadcrumb-item>
    </el-breadcrumb>

    <el-table :data="entries" v-loading="loading" stripe @row-dblclick="openEntry" style="width: 100%">
      <el-table-column label="Name" min-width="300">
        <template #default="{ row }">
          <span :class="{ 'dir-icon': row.is_dir, 'file-icon': !row.is_dir }">
            {{ row.is_dir ? '📁' : '📄' }} {{ row.name }}
          </span>
        </template>
      </el-table-column>
      <el-table-column prop="size" label="Size" width="100">
        <template #default="{ row }">{{ row.is_dir ? '-' : formatSize(row.size) }}</template>
      </el-table-column>
      <el-table-column prop="permissions" label="Perms" width="90" />
      <el-table-column prop="modified_at" label="Modified" width="180">
        <template #default="{ row }">{{ formatDate(row.modified_at) }}</template>
      </el-table-column>
      <el-table-column label="Actions" width="280" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="editFile(row)" :disabled="row.is_dir">Edit</el-button>
          <el-button size="small" @click="showRename(row)">Rename</el-button>
          <el-button size="small" @click="showChmod(row)">Chmod</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">Delete</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="showEdit" title="Edit File" width="800" top="5vh">
      <el-input v-model="editContent" type="textarea" :rows="20" style="font-family: monospace" />
      <template #footer>
        <el-button @click="showEdit = false">Cancel</el-button>
        <el-button type="primary" @click="saveEdit">Save</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showCreateFile" title="Create File" width="400">
      <el-form :model="createForm" label-width="80">
        <el-form-item label="Path">
          <el-input v-model="createForm.path" :placeholder="currentPath + '/filename'" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateFile = false">Cancel</el-button>
        <el-button type="primary" @click="handleCreateFile">Create</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showCreateDir" title="Create Directory" width="400">
      <el-form :model="createForm" label-width="80">
        <el-form-item label="Path">
          <el-input v-model="createForm.path" :placeholder="currentPath + '/dirname'" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateDir = false">Cancel</el-button>
        <el-button type="primary" @click="handleCreateDir">Create</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showRenameDialog" title="Rename" width="400">
      <el-form :model="renameForm" label-width="80">
        <el-form-item label="New Name">
          <el-input v-model="renameForm.newName" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRenameDialog = false">Cancel</el-button>
        <el-button type="primary" @click="handleRename">Rename</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showChmodDialog" title="Change Permissions" width="400">
      <el-form :model="chmodForm" label-width="80">
        <el-form-item label="Mode">
          <el-input v-model="chmodForm.mode" placeholder="e.g. 755" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showChmodDialog = false">Cancel</el-button>
        <el-button type="primary" @click="handleChmod">Apply</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showUpload" title="Upload Files" width="400">
      <el-upload drag :auto-upload="false" :on-change="handleUploadChange" multiple>
        <div class="el-upload__text">Drop files here or click to browse</div>
      </el-upload>
      <div v-if="uploadFiles.length > 0" style="margin-top: 8px">
        <el-tag v-for="(f, i) in uploadFiles" :key="i" closable @close="uploadFiles.splice(i, 1)" style="margin: 2px">{{ f.name }}</el-tag>
      </div>
      <template #footer>
        <el-button @click="showUpload = false">Cancel</el-button>
        <el-button type="primary" @click="handleUpload">Upload ({{ uploadFiles.length }})</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listFiles, readFile, writeFile, createFile, createDir, deleteFile, renameFile, chmodFile, uploadFile, downloadFile } from '@/api/files'
import type { FileInfo } from '@/types'

const currentPath = ref('/')
const entries = ref<FileInfo[]>([])
const loading = ref(false)
const breadcrumbs = computed(() => {
  const parts = currentPath.value.replace(/^\/|\/$/g, '').split('/').filter(Boolean)
  const crumbs = [{ name: '/', path: '/' }]
  let acc = ''
  for (const p of parts) {
    acc += '/' + p
    crumbs.push({ name: p, path: acc })
  }
  return crumbs
})

const showEdit = ref(false)
const editContent = ref('')
const editPath = ref('')

const createForm = ref({ path: '' })
const showCreateFile = ref(false)
const showCreateDir = ref(false)

const renameForm = ref({ oldPath: '', newName: '' })
const showRenameDialog = ref(false)

const chmodForm = ref({ path: '', mode: '755' })
const showChmodDialog = ref(false)

const showUpload = ref(false)
const uploadFiles = ref<File[]>([])

async function fetch() {
  loading.value = true
  try {
    const res = await listFiles(currentPath.value)
    entries.value = res.data
  } catch {
    ElMessage.error('Failed to list directory')
  } finally {
    loading.value = false
  }
}

function goTo(path: string) {
  currentPath.value = path
  fetch()
}

function openEntry(row: FileInfo) {
  if (row.is_dir) {
    currentPath.value = row.path
    fetch()
  } else {
    editPath.value = row.path
    readFile(row.path).then(res => {
      editContent.value = res.data
      showEdit.value = true
    }).catch(() => ElMessage.error('Failed to read file'))
  }
}

function editFile(row: FileInfo) {
  if (!row.is_dir) openEntry(row)
}

function saveEdit() {
  writeFile(editPath.value, editContent.value).then(() => {
    ElMessage.success('File saved')
    showEdit.value = false
  }).catch(() => ElMessage.error('Failed to save'))
}

function handleCreateFile() {
  const path = createForm.value.path.startsWith('/') ? createForm.value.path : `${currentPath.value}/${createForm.value.path}`
  createFile(path).then(() => {
    ElMessage.success('File created')
    showCreateFile.value = false
    createForm.value.path = ''
    fetch()
  }).catch(() => ElMessage.error('Failed to create file'))
}

function handleCreateDir() {
  const path = createForm.value.path.startsWith('/') ? createForm.value.path : `${currentPath.value}/${createForm.value.path}`
  createDir(path).then(() => {
    ElMessage.success('Directory created')
    showCreateDir.value = false
    createForm.value.path = ''
    fetch()
  }).catch(() => ElMessage.error('Failed to create directory'))
}

function showRename(row: FileInfo) {
  renameForm.value = { oldPath: row.path, newName: row.name }
  showRenameDialog.value = true
}

function handleRename() {
  const newPath = renameForm.value.oldPath.replace(/[^/]+$/, renameForm.value.newName)
  renameFile(renameForm.value.oldPath, newPath).then(() => {
    ElMessage.success('Renamed')
    showRenameDialog.value = false
    fetch()
  }).catch(() => ElMessage.error('Failed to rename'))
}

function showChmod(row: FileInfo) {
  chmodForm.value = { path: row.path, mode: row.permissions }
  showChmodDialog.value = true
}

function handleChmod() {
  chmodFile(chmodForm.value.path, chmodForm.value.mode).then(() => {
    ElMessage.success('Permissions changed')
    showChmodDialog.value = false
    fetch()
  }).catch(() => ElMessage.error('Failed to chmod'))
}

async function handleDelete(row: FileInfo) {
  try {
    await ElMessageBox.confirm(`Delete ${row.is_dir ? 'directory' : 'file'} ${row.name}?`, 'Confirm')
    await deleteFile(row.path, row.is_dir)
    ElMessage.success('Deleted')
    await fetch()
  } catch { /* cancelled or failed */ }
}

function handleUploadChange(file: any) {
  const raw = file.raw || file
  if (raw instanceof File) {
    uploadFiles.value = [...uploadFiles.value, raw]
  }
}

async function handleUpload() {
  for (const file of uploadFiles.value) {
    try {
      await uploadFile(currentPath.value, file.name, file)
      ElMessage.success(`Uploaded ${file.name}`)
    } catch {
      ElMessage.error(`Failed to upload ${file.name}`)
    }
  }
  uploadFiles.value = []
  showUpload.value = false
  await fetch()
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleString()
}

onMounted(fetch)
</script>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: center; }
h2 { margin: 0; }
.actions { display: flex; gap: 8px; }
.dir-icon { cursor: pointer; }
.file-icon { cursor: pointer; }
</style>
