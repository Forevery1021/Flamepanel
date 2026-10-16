<template>
  <FpModal v-model="visible" :header="t('node.nodeFiles')" size="xl">
    <div class="modal-form">
      <div v-if="nodeName" class="text-muted text-xs">{{ t('node.name') }}: {{ nodeName }}</div>
      <div class="files-path-bar">
        <FpButton variant="ghost" size="small" @click="emit('up')">{{ t('node.up') }}</FpButton>
        <FpInput v-model="path" :placeholder="t('node.pathPlaceholder')" class="flex-1" />
        <FpButton variant="primary" size="small" @click="emit('load')">{{ t('node.go') }}</FpButton>
        <FpButton variant="ghost" size="small" @click="emit('load')">{{
          t('node.refresh')
        }}</FpButton>
      </div>
      <div v-if="entries.length" class="files-list">
        <div
          v-for="e in entries"
          :key="e.name"
          class="files-row"
          :role="e.is_dir ? 'button' : undefined"
          :tabindex="e.is_dir ? 0 : undefined"
          @click="e.is_dir && emit('enter', e.name)"
          @keydown.enter.self="e.is_dir && emit('enter', e.name)"
          @keydown.space.self.prevent="e.is_dir && emit('enter', e.name)"
        >
          <span class="files-icon"><i :class="e.is_dir ? 'oi oi-folder' : 'oi oi-file'" /></span>
          <span class="files-name cell-truncate">{{ e.name }}</span>
          <span class="text-muted text-xs">{{ e.is_dir ? '—' : formatSize(e.size) }}</span>
          <div class="files-actions">
            <FpButton
              v-if="!e.is_dir"
              variant="link"
              size="small"
              @click.stop="emit('download', e.name)"
            >
              {{ t('node.download') }}
            </FpButton>
          </div>
        </div>
        <div v-if="!entries.length" class="text-muted text-xs">—</div>
      </div>
      <div v-else class="text-muted text-xs">—</div>
      <FpInput v-model="uploadContent" :label="t('node.uploadFile')" />
      <div class="files-upload-bar">
        <FpInput
          v-model="uploadName"
          placeholder="filename.txt"
          :aria-label="t('node.uploadFile')"
          class="flex-1"
        />
        <FpButton variant="primary" size="small" :loading="uploading" @click="emit('upload')">
          {{ t('node.upload') }}
        </FpButton>
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.close') }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { RemoteFileEntry } from '@/types'
import { formatSize } from '@/utils/format'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 远程文件弹窗（Doc/23 §3.3 T2-4 视图拆分第七站 · Stage5 远程文件）。
 *
 * 五类操作全部上抛（进入 / 上跳 / 跳转 / 刷新 / 下载 / 上传），路径与文件清单
 * 由数据层持有——**路径是本弹窗的「当前位置」而非输入框的临时值**：上跳与
 * 进入都会改写它，用户手改后点「进入」才生效（改了就自动跳会让路径栏在输入
 * 过程中不停重拉目录）。
 *
 * 目录行渲染 `—` 而不是 `0 B`：目录没有「大小」这个属性，显示 0 会让人以为
 * 是个空文件（原实现口径，下沉为 `e.is_dir ? '—' : formatSize(e.size)`）。
 *
 * 关闭按钮只有「关闭」：本弹窗是**浏览视图**，没有需要确认或保存的改动。
 */
withDefaults(
  defineProps<{
    /** 目标节点名（仅用于标题下方提示） */
    nodeName?: string
    /** 当前目录条目 */
    entries?: RemoteFileEntry[]
    /** 上传中 */
    uploading?: boolean
  }>(),
  { nodeName: '', entries: () => [], uploading: false },
)

const visible = defineModel<boolean>({ required: true })
const path = defineModel<string>('path', { default: '.' })
const uploadName = defineModel<string>('uploadName', { default: '' })
const uploadContent = defineModel<string>('uploadContent', { default: '' })

const emit = defineEmits<{
  enter: [name: string]
  up: []
  load: []
  download: [name: string]
  upload: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.cell-truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.files-path-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.files-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  max-height: 300px;
  overflow-y: auto;
}
.files-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--fp-border);
  cursor: default;
}
.files-row:hover {
  background: var(--fp-bg-hover);
}
.files-icon {
  width: 20px;
  text-align: center;
}
.files-name {
  flex: 1;
}
.files-actions {
  display: flex;
  gap: 4px;
}
.files-upload-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.flex-1 {
  flex: 1;
}
</style>
