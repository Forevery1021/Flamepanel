<template>
  <div class="view-container">
    <div class="page-toolbar">
      <FpButton v-permission="{ perm: 'file:create', mode: 'view' }" variant="primary" icon="oi oi-upload" @click="showUpload = true">
        {{ t('file.upload') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'file:create', mode: 'view' }" variant="ghost" icon="oi oi-file-plus" @click="showCreateFile = true">
        {{ t('file.createFile') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'file:create', mode: 'view' }" variant="ghost" icon="oi oi-folder-plus" @click="showCreateDir = true">
        {{ t('file.createDir') }}
      </FpButton>
      <FpButton variant="ghost" icon="oi oi-refresh" :loading="loading" @click="fetch">
        {{ t('common.refresh') }}
      </FpButton>
    </div>

    <div class="panel file-panel">
      <FpBreadcrumb :model="breadcrumbItems" class="file-breadcrumb" />

      <FpStatePanel
        :loading="loading"
        :error="fetchError"
        :empty="!entries.length"
        retryable
        :empty-title="t('common.noData')"
        :empty-desc="t('file.emptyDir')"
        @retry="fetch"
      >
        <FpTable
          :rows="entries"
          :loading="loading"
          :empty-text="t('common.noData')"
          striped-rows
          virtual
          virtual-scroll-height="560px"
          @row-dblclick="onRowDblClick"
        >
          <FpColumn field="name" :header="t('file.name')" style="min-width: 300px">
            <template #body="{ data }">
              <span class="file-name" :class="data.is_dir ? 'dir-icon' : 'file-icon'">
                <i :class="data.is_dir ? 'oi oi-folder' : 'oi oi-file'" />
                {{ data.name }}
              </span>
            </template>
          </FpColumn>
          <FpColumn :header="t('file.size')" style="width: 100px">
            <template #body="{ data }">
              {{ data.is_dir ? '-' : formatSize(data.size) }}
            </template>
          </FpColumn>
          <FpColumn field="permissions" :header="t('file.permissions')" style="width: 90px" />
          <FpColumn :header="t('file.modified')" style="width: 180px">
            <template #body="{ data }">
              {{ formatDate(data.modified_at) }}
            </template>
          </FpColumn>
          <FpColumn :header="t('file.actions')" style="width: 280px" frozen>
            <template #body="{ data }">
              <div class="row-actions">
                <FpButton v-permission="{ perm: 'file:update', mode: 'view' }" variant="link" :disabled="data.is_dir" @click="editFile(data)">
                  {{ t('file.edit') }}
                </FpButton>
                <FpButton v-permission="{ perm: 'file:update', mode: 'view' }" variant="link" @click="showRename(data)">{{ t('file.rename') }}</FpButton>
                <FpButton v-permission="{ perm: 'file:update', mode: 'view' }" variant="link" @click="showChmod(data)">{{ t('file.chmod') }}</FpButton>
                <FpButton v-permission="{ perm: 'file:delete', mode: 'view' }" variant="link" @click="handleDelete(data)">{{ t('file.delete') }}</FpButton>
              </div>
            </template>
          </FpColumn>
        </FpTable>
      </FpStatePanel>
    </div>

    <FpModal v-model="showEdit" :header="t('file.edit')" width="800">
      <FpTextarea v-model="editContent" rows="20" class="mono w-full" />
      <template #footer>
        <FpButton variant="ghost" @click="showEdit = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" @click="saveEdit">{{ t('file.save') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showCreateFile" :header="t('file.createFile')">
      <div class="modal-form">
        <FpInput
          v-model="createForm.path"
          :label="t('file.path')"
          :placeholder="currentPath + '/filename'"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showCreateFile = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" @click="handleCreateFile">{{ t('file.createFile') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showCreateDir" :header="t('file.createDir')">
      <div class="modal-form">
        <FpInput
          v-model="createForm.path"
          :label="t('file.path')"
          :placeholder="currentPath + '/dirname'"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showCreateDir = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" @click="handleCreateDir">{{ t('file.createDir') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showRenameDialog" :header="t('file.rename')">
      <div class="modal-form">
        <FpInput v-model="renameForm.newName" :label="t('file.newName')" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showRenameDialog = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" @click="handleRename">{{ t('file.rename') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showChmodDialog" :header="t('file.chmod')">
      <div class="modal-form">
        <FpInput
          v-model="chmodForm.mode"
          :label="t('file.mode')"
          :placeholder="t('file.mode')"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showChmodDialog = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" @click="handleChmod">{{ t('common.confirm') }}</FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showUpload" :header="t('file.upload')">
      <div class="modal-form">
        <FpFileUpload
          mode="basic"
          :auto="false"
          :multiple="true"
          :custom-upload="true"
          :choose-label="t('file.uploadHere')"
          @select="handleUploadSelect"
          @uploader="handleUpload"
        />
        <div v-if="uploadFiles.length > 0" class="upload-list">
          <FpChip
            v-for="(f, i) in uploadFiles"
            :key="i"
            :label="f.name"
            :removable="true"
            class="upload-chip"
            @remove="uploadFiles.splice(i, 1)"
          />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showUpload = false">{{ t('file.cancel') }}</FpButton>
        <FpButton variant="primary" :disabled="uploadFiles.length === 0" @click="handleUpload">
          {{ t('file.upload') }} ({{ uploadFiles.length }})
        </FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'





import {
  listFiles,
  readFile,
  writeFile,
  createFile,
  createDir,
  deleteFile,
  renameFile,
  chmodFile,
  uploadFile,
} from '@/api/files'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type { FileInfo } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpBreadcrumb from '@/components/ui/FpBreadcrumb.vue'
import FpChip from '@/components/ui/FpChip.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpFileUpload from '@/components/ui/FpFileUpload.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const queryClient = useQueryCacheClient()

const currentPath = ref('/')
const breadcrumbItems = computed(() => {
  const parts = currentPath.value
    .replace(/^\/|\/$/g, '')
    .split('/')
    .filter(Boolean)
  const crumbs: Array<{ label: string; command: () => void }> = [
    { label: '/', command: () => goTo('/') },
  ]
  let acc = ''
  for (const p of parts) {
    acc += '/' + p
    crumbs.push({ label: p, command: () => goTo(acc) })
  }
  return crumbs
})

// F1.1：文件列表按路径缓存；写操作完成后 invalidate 对应路径
const filesQuery = useApiQuery<FileInfo[]>(
  () => queryKeys.files.list(currentPath.value),
  () => listFiles(currentPath.value),
)
const entries = computed<FileInfo[]>(() => filesQuery.data.value ?? [])
const loading = filesQuery.loading
const fetchError = filesQuery.error

async function fetch() {
  await filesQuery.refresh()
}

/** 写操作后刷新当前目录（invalidate） */
function invalidateCurrentDir() {
  void queryClient.invalidateQueries({ queryKey: ['files', 'list'] })
}

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

function goTo(path: string) {
  currentPath.value = path
}

function onRowDblClick(e: { data: FileInfo }) {
  openEntry(e.data)
}

function openEntry(row: FileInfo) {
  if (row.is_dir) {
    currentPath.value = row.path
  } else {
    editPath.value = row.path
    readFile(row.path)
      .then((res) => {
        editContent.value = res.data
        showEdit.value = true
      })
      .catch(() => toast.error(t('common.failed')))
  }
}

function editFile(row: FileInfo) {
  if (!row.is_dir) openEntry(row)
}

function saveEdit() {
  writeFile(editPath.value, editContent.value)
    .then(() => {
      toast.success(t('common.success'))
      showEdit.value = false
    })
    .catch(() => toast.error(t('common.failed')))
}

function handleCreateFile() {
  const path = createForm.value.path.startsWith('/')
    ? createForm.value.path
    : `${currentPath.value}/${createForm.value.path}`
  createFile(path)
    .then(() => {
      toast.success(t('common.success'))
      showCreateFile.value = false
      createForm.value.path = ''
      invalidateCurrentDir()
    })
    .catch(() => toast.error(t('common.failed')))
}

function handleCreateDir() {
  const path = createForm.value.path.startsWith('/')
    ? createForm.value.path
    : `${currentPath.value}/${createForm.value.path}`
  createDir(path)
    .then(() => {
      toast.success(t('common.success'))
      showCreateDir.value = false
      createForm.value.path = ''
      invalidateCurrentDir()
    })
    .catch(() => toast.error(t('common.failed')))
}

function showRename(row: FileInfo) {
  renameForm.value = { oldPath: row.path, newName: row.name }
  showRenameDialog.value = true
}

function handleRename() {
  const newPath = renameForm.value.oldPath.replace(/[^/]+$/, renameForm.value.newName)
  renameFile(renameForm.value.oldPath, newPath)
    .then(() => {
      toast.success(t('common.success'))
      showRenameDialog.value = false
      invalidateCurrentDir()
    })
    .catch(() => toast.error(t('common.failed')))
}

function showChmod(row: FileInfo) {
  chmodForm.value = { path: row.path, mode: row.permissions }
  showChmodDialog.value = true
}

function handleChmod() {
  chmodFile(chmodForm.value.path, chmodForm.value.mode)
    .then(() => {
      toast.success(t('common.success'))
      showChmodDialog.value = false
      invalidateCurrentDir()
    })
    .catch(() => toast.error(t('common.failed')))
}

function handleDelete(row: FileInfo) {
  confirmAction({
    message: t('file.deleteConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await deleteFile(row.path, row.is_dir)
        toast.success(t('common.success'))
        invalidateCurrentDir()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
}

function handleUploadSelect(e: { files: File[] }) {
  uploadFiles.value = [...uploadFiles.value, ...e.files]
}

async function handleUpload() {
  for (const file of uploadFiles.value) {
    try {
      await uploadFile(currentPath.value, file.name, file)
      toast.success(t('common.success'))
    } catch {
      toast.error(t('common.failed'))
    }
  }
  uploadFiles.value = []
  showUpload.value = false
  invalidateCurrentDir()
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 ' + t('file.bytes')
  const k = 1024
  const sizes = [t('file.bytes'), t('file.kb'), t('file.mb'), t('file.gb'), 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleString()
}
</script>

<style scoped>
.page-toolbar {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-4);
}
.file-panel {
  overflow: hidden;
}
.file-breadcrumb {
  margin-bottom: var(--fp-space-3);
}
.file-name {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.file-name i {
  font-size: 14px;
}
.dir-icon i {
  color: var(--fp-warning);
}
.file-icon i {
  color: var(--fp-info);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.upload-list {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
}
.mono {
  font-family: var(--fp-font-mono);
}
</style>
