<template>
  <LayoutContent :title="t('nav.files')" :description="t('pageDesc.files')">
    <template #actions>
      <FpButton
        v-permission="{ perm: 'file:upload', mode: 'view' }"
        variant="primary"
        icon="oi oi-upload"
        @click="showUpload = true"
      >
        {{ t('file.upload') }}
      </FpButton>
      <FpButton
        v-permission="{ perm: 'file:write', mode: 'view' }"
        variant="ghost"
        icon="oi oi-file-plus"
        @click="openCreate('file')"
      >
        {{ t('file.createFile') }}
      </FpButton>
      <FpButton
        v-permission="{ perm: 'file:write', mode: 'view' }"
        variant="ghost"
        icon="oi oi-folder-plus"
        @click="openCreate('dir')"
      >
        {{ t('file.createDir') }}
      </FpButton>
      <FpButton variant="ghost" icon="oi oi-trash" @click="showRecycle = true">
        {{ t('file.recycleBin') }}
      </FpButton>
      <FpButton
        variant="ghost"
        icon="oi oi-refresh"
        :loading="loading"
        :aria-label="t('common.refresh')"
        :title="t('common.refresh')"
        @click="refresh"
      />
    </template>

    <div class="panel file-panel">
      <div class="file-toolbar">
        <FpButton
          variant="ghost"
          icon="oi oi-arrow-up"
          :disabled="currentPath === '/'"
          :aria-label="t('file.parentDir')"
          :title="t('file.parentDir')"
          @click="goTo(parentOf(currentPath))"
        />
        <FpBreadcrumb :items="breadcrumbItems" class="file-breadcrumb" />
      </div>

      <FileSelectionBar
        v-if="selected.length"
        :count="selected.length"
        @compress="openCompress(selected)"
        @copy="openTransfer('copy', selected)"
        @move="openTransfer('move', selected)"
        @delete="confirmDelete(selectedRows)"
        @clear="clearSelection"
      />

      <FpStatePanel
        :loading="loading"
        :error="fetchError"
        :empty="!entries.length"
        retryable
        :empty-title="t('common.noData')"
        :empty-desc="t('file.emptyDir')"
        @retry="refresh"
      >
        <FileTable
          :entries="entries"
          :loading="loading"
          :selected="selected"
          :all-selected="allSelected"
          :downloading="downloading"
          @open="openEntry"
          @edit="openEditor"
          @download="download"
          @action="onRowAction"
          @toggle="toggle"
          @toggle-all="toggleAll"
        />
      </FpStatePanel>
    </div>

    <FileEditorModal v-model="showEdit" :file="editing" @saved="invalidate" />
    <FileUploadModal v-model="showUpload" :dir="currentPath" @uploaded="invalidate" />
    <FileCompressModal v-model="showCompress" :paths="compressPaths" @done="afterBatch" />
    <FileExtractModal v-model="showExtract" :path="extractPath" @done="invalidate" />
    <FileTransferModal
      v-model="showTransfer"
      :paths="transferPaths"
      :mode="transferMode"
      :dir="currentPath"
      @done="afterBatch"
    />
    <FileRecycleDrawer v-model="showRecycle" @restored="invalidate" />
    <FileCreateModal
      v-model="showCreate"
      :kind="createKind"
      :dir="currentPath"
      @done="invalidate"
    />
    <FileRenameModal v-model="showRename" :file="target" @done="invalidate" />
    <FileChmodModal v-model="showChmod" :file="target" @done="invalidate" />
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { TransferMode } from '@/api/files'
import type { FileInfo } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpBreadcrumb from '@/components/ui/FpBreadcrumb.vue'
import FileTable from '@/components/files/FileTable.vue'
import FileSelectionBar from '@/components/files/FileSelectionBar.vue'
import FileEditorModal from '@/components/files/FileEditorModal.vue'
import FileUploadModal from '@/components/files/FileUploadModal.vue'
import FileCompressModal from '@/components/files/FileCompressModal.vue'
import FileExtractModal from '@/components/files/FileExtractModal.vue'
import FileTransferModal from '@/components/files/FileTransferModal.vue'
import FileRecycleDrawer from '@/components/files/FileRecycleDrawer.vue'
import FileCreateModal from '@/components/files/FileCreateModal.vue'
import FileRenameModal from '@/components/files/FileRenameModal.vue'
import FileChmodModal from '@/components/files/FileChmodModal.vue'
import { parentOf } from '@/components/files/fileUtils'
import { useFileBrowser } from '@/composables/useFileBrowser'
import { useFileOps } from '@/composables/useFileOps'
import { entryAction, type FileRowActionKey } from '@/utils/fileBrowser'

/**
 * 文件管理页：只负责编排。
 * 数据层 `useFileBrowser`（目录 / 列表 / 多选）+ 动作 `useFileOps`（删除 / 下载）；
 * 表格、选择栏与各对话框均为独立组件，对话框自行调用接口并在完成后 `done`。
 */
const { t } = useI18n()

const {
  currentPath,
  crumbs,
  entries,
  loading,
  error: fetchError,
  refresh,
  invalidate,
  goTo,
  selected,
  selectedRows,
  allSelected,
  toggle,
  toggleAll,
  clearSelection,
} = useFileBrowser()

function afterBatch() {
  clearSelection()
  invalidate()
}

const { downloading, download, confirmDelete, confirmPurge } = useFileOps(afterBatch)

const breadcrumbItems = computed(() =>
  crumbs.value.map((c) => ({ label: c.label, command: () => goTo(c.path) })),
)

// ── 对话框状态 ──
const showEdit = ref(false)
const editing = ref<FileInfo | null>(null)
const showUpload = ref(false)
const showRecycle = ref(false)
const showCompress = ref(false)
const compressPaths = ref<string[]>([])
const showExtract = ref(false)
const extractPath = ref('')
const showTransfer = ref(false)
const transferPaths = ref<string[]>([])
const transferMode = ref<TransferMode>('copy')
const showCreate = ref(false)
const createKind = ref<'file' | 'dir'>('file')
const showRename = ref(false)
const showChmod = ref(false)
const target = ref<FileInfo | null>(null)

function openEditor(row: FileInfo) {
  editing.value = row
  showEdit.value = true
}

function openCompress(paths: string[]) {
  compressPaths.value = [...paths]
  showCompress.value = true
}

function openExtract(row: FileInfo) {
  extractPath.value = row.path
  showExtract.value = true
}

function openTransfer(mode: TransferMode, paths: string[]) {
  transferMode.value = mode
  transferPaths.value = [...paths]
  showTransfer.value = true
}

function openCreate(kind: 'file' | 'dir') {
  createKind.value = kind
  showCreate.value = true
}

function openEntry(row: FileInfo) {
  const action = entryAction(row)
  if (action === 'enter') goTo(row.path)
  else if (action === 'extract') openExtract(row)
  else openEditor(row)
}

function onRowAction(key: FileRowActionKey, row: FileInfo) {
  switch (key) {
    case 'rename':
      target.value = row
      showRename.value = true
      break
    case 'chmod':
      target.value = row
      showChmod.value = true
      break
    case 'compress':
      openCompress([row.path])
      break
    case 'copy':
    case 'move':
      openTransfer(key, [row.path])
      break
    case 'extract':
      openExtract(row)
      break
    case 'delete':
      confirmDelete([row])
      break
    case 'purge':
      confirmPurge(row)
      break
  }
}
</script>

<style scoped>
.file-panel {
  overflow: hidden;
}
.file-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-3);
}
.file-breadcrumb {
  flex: 1;
  min-width: 0;
}
</style>
