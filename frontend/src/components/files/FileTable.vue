<template>
  <FpTable
    :rows="entries"
    :loading="loading"
    :empty-text="t('common.noData')"
    striped-rows
    virtual
    virtual-scroll-height="560px"
    class="file-table"
    @row-dblclick="(e: { data: FileInfo }) => emit('open', e.data)"
  >
    <FpColumn style="width: 44px">
      <template #header>
        <FpCheckbox
          :model-value="allSelected"
          :aria-label="t('file.selectAll')"
          @update:model-value="(v: boolean) => emit('toggleAll', v)"
        />
      </template>
      <template #body="{ data }">
        <FpCheckbox
          :model-value="selected.includes(data.path)"
          :aria-label="t('file.selectRow', { name: data.name })"
          @update:model-value="(v: boolean) => emit('toggle', data.path, v)"
        />
      </template>
    </FpColumn>
    <FpColumn field="name" :header="t('file.name')" class="col-name">
      <template #body="{ data }">
        <button
          type="button"
          class="file-name"
          :class="data.is_dir ? 'dir-icon' : 'file-icon'"
          @click="emit('open', data)"
        >
          <i :class="entryIcon(data)" aria-hidden="true" />
          <span class="file-name-text">{{ data.name }}</span>
          <FpTag v-if="data.is_symlink" :value="t('file.symlink')" severity="neutral" />
        </button>
      </template>
    </FpColumn>
    <FpColumn :header="t('file.size')" class="col-size">
      <template #body="{ data }">
        {{ data.is_dir ? '-' : formatSize(data.size) }}
      </template>
    </FpColumn>
    <FpColumn field="permissions" :header="t('file.permissions')" class="col-perm" />
    <FpColumn :header="t('file.modified')" class="col-modified">
      <template #body="{ data }">
        {{ formatModified(data.modified_at) }}
      </template>
    </FpColumn>
    <FpColumn :header="t('file.actions')" class="col-actions" frozen>
      <template #body="{ data }">
        <div class="row-actions">
          <FpButton
            v-if="!data.is_dir"
            v-permission="{ perm: 'file:read', mode: 'view' }"
            variant="link"
            @click="emit('edit', data)"
          >
            {{ t('file.edit') }}
          </FpButton>
          <FpButton
            v-if="!data.is_dir"
            v-permission="{ perm: 'file:upload', mode: 'view' }"
            variant="link"
            :loading="downloading === data.path"
            @click="emit('download', data)"
          >
            {{ t('file.download') }}
          </FpButton>
          <FpActionMenu
            :items="menuItems(data)"
            :aria-label="t('file.moreFor', { name: data.name })"
          />
        </div>
      </template>
    </FpColumn>
  </FpTable>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpActionMenu, { type FpActionItem } from '@/components/ui/FpActionMenu.vue'
import type { FileInfo } from '@/types'
import { formatSize } from '@/utils/format'
import {
  entryIcon,
  formatModified,
  rowActionKeys,
  type FileRowActionKey,
} from '@/utils/fileBrowser'

defineProps<{
  entries: FileInfo[]
  loading: boolean
  selected: string[]
  allSelected: boolean
  downloading: string
}>()

const emit = defineEmits<{
  open: [row: FileInfo]
  edit: [row: FileInfo]
  download: [row: FileInfo]
  action: [key: FileRowActionKey, row: FileInfo]
  toggle: [path: string, on: boolean]
  toggleAll: [on: boolean]
}>()

const { t } = useI18n()

const ACTION_META: Record<
  FileRowActionKey,
  { label: string; icon: string; danger?: boolean; separatorBefore?: boolean }
> = {
  rename: { label: 'file.rename', icon: 'oi oi-pencil' },
  chmod: { label: 'file.chmod', icon: 'oi oi-lock' },
  compress: { label: 'file.compress', icon: 'oi oi-box' },
  copy: { label: 'file.copyTo', icon: 'oi oi-copy' },
  move: { label: 'file.moveTo', icon: 'oi oi-arrow-right' },
  extract: { label: 'file.extract', icon: 'oi oi-folder-open' },
  delete: { label: 'file.delete', icon: 'oi oi-trash', danger: true, separatorBefore: true },
  purge: { label: 'file.permanentDelete', icon: 'oi oi-times', danger: true },
}

function menuItems(row: FileInfo): FpActionItem[] {
  return rowActionKeys(row).map((key) => {
    const meta = ACTION_META[key]
    return {
      key,
      label: t(meta.label),
      icon: meta.icon,
      perm: 'file:write',
      danger: meta.danger,
      separatorBefore: meta.separatorBefore,
      command: () => emit('action', key, row),
    }
  })
}
</script>

<style scoped>
.file-table :deep(.col-name) {
  min-width: 280px;
}
.file-table :deep(.col-size) {
  width: 100px;
}
.file-table :deep(.col-perm) {
  width: 90px;
}
.file-table :deep(.col-modified) {
  width: 180px;
}
.file-table :deep(.col-actions) {
  width: 200px;
}
/* 平板（768–1024）：让出名称列空间——权限列隐藏，修改时间收窄 */
@media (max-width: 1024px) {
  .file-table :deep(.col-name) {
    min-width: 200px;
  }
  .file-table :deep(.col-perm) {
    display: none;
  }
  .file-table :deep(.col-modified) {
    width: 150px;
  }
}
.file-name {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-2);
  max-width: 100%;
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.file-name:hover .file-name-text,
.file-name:focus-visible .file-name-text {
  color: var(--fp-brand);
  text-decoration: underline;
}
.file-name:focus-visible {
  outline: none;
  box-shadow: var(--fp-focus-ring);
  border-radius: var(--fp-radius-sm);
}
.file-name-text {
  overflow-wrap: break-word;
  min-width: 0;
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
  align-items: center;
  gap: var(--fp-space-2);
}
</style>
