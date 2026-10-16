<template>
  <div class="panel">
    <FpTable :rows="rows" :loading="loading" :paginator="false" :empty-text="t('common.noData')" :empty-desc="t('pageDesc.emptyHint')">
      <template v-if="$slots.emptyAction" #emptyAction><slot name="emptyAction" /></template>
      <FpColumn field="filename" :header="t('backup.filename')" style="min-width: 260px" />
      <FpColumn :header="t('backup.size')" style="width: 120px">
        <template #body="{ data }">
          {{ formatSize(data.size) }}
        </template>
      </FpColumn>
      <FpColumn field="created_at" :header="t('backup.createdAt')" style="width: 180px">
        <template #body="{ data }">
          <span class="mono">{{ data.created_at }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.colActions')" style="width: 240px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton variant="primary" @click="emit('download', data.filename)">
              {{ t('backup.download') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'backup:update', mode: 'view' }"
              variant="warning"
              @click="emit('restore', data.filename)"
            >
              {{ t('backup.restore') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'backup:delete', mode: 'view' }"
              variant="danger"
              @click="emit('remove', data.filename)"
            >
              {{ t('backup.delete') }}
            </FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { BackupEntry } from '@/api/backups'
import { formatSize } from '@/utils/format'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 本地备份列表（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * **纯展示 + 事件上抛**：下载 / 恢复 / 删除三类行内操作全部交由父级编排
 * （`useBackupLocal`）。三列的取值语义各有一条易漏的细节，均由本组件承担：
 * 体积走统一 `formatSize`（不是后端字节数直出）、时间原样等宽字体
 * （后端给的就是可读串，前端再 `toLocaleString` 一次会与文案标称不一致）、
 * 下载按钮**不带权限指令**（下载是只读操作，原实现如此）。
 */
withDefaults(
  defineProps<{
    rows: BackupEntry[]
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  download: [filename: string]
  restore: [filename: string]
  remove: [filename: string]
}>()

const { t } = useI18n()
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.mono {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
</style>
