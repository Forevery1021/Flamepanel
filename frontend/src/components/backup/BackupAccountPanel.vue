<template>
  <div class="panel">
    <FpTable :rows="rows" :loading="loading" :paginator="false" :empty-text="t('common.noData')" :empty-desc="t('pageDesc.emptyHint')">
      <template v-if="$slots.emptyAction" #emptyAction><slot name="emptyAction" /></template>
      <FpColumn field="name" :header="t('backup.accountName')" style="min-width: 160px" />
      <FpColumn :header="t('backup.accountType')" style="width: 100px">
        <template #body="{ data }">
          <FpTag severity="info" :value="String(data.account_type).toUpperCase()" />
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.accountEnabled')" style="width: 90px">
        <template #body="{ data }">
          <FpSwitch
            :model-value="data.enabled"
            @update:model-value="(v: boolean) => emit('toggle', data, v)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.accountTestStatus')" style="width: 120px">
        <template #body="{ data }">
          <FpTag
            :severity="testStatusTag(data.last_test_status)"
            :value="testStatusLabel(data.last_test_status, t)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.accountTestedAt')" style="width: 170px">
        <template #body="{ data }">
          <span>{{ formatDateTime(data.last_test_at) }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.colActions')" style="width: 280px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'backup:update', mode: 'view' }"
              variant="primary"
              :loading="testingId === data.id"
              @click="emit('test', data)"
            >
              {{ t('backup.accountTest') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'backup:update', mode: 'view' }"
              variant="link"
              @click="emit('edit', data)"
            >
              {{ t('common.edit') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'backup:delete', mode: 'view' }"
              variant="link"
              @click="emit('remove', data)"
            >
              {{ t('common.delete') }}
            </FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { BackupAccountDto } from '@/api/generated'
import { formatDateTime, testStatusLabel, testStatusTag } from '@/utils/backupFormat'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'

/**
 * 备份账号列表（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 账号表的「启用」开关是**行内直接可点的开关**（不是进弹窗改），因此它必须
 * 上抛 `(row, value)` 两个参数——数据层要用行对象做二次确认、用 value 决定
 * 文案是「启用」还是「停用」。`testingId` 只锁当前行（其余行的测试按钮不禁用，
 * 拆开看这是原实现就有的一条细粒度：测试接口互不影响）。
 */
withDefaults(
  defineProps<{
    rows: BackupAccountDto[]
    loading?: boolean
    /** 正在测试连接的行 id（只锁该行） */
    testingId?: number | null
  }>(),
  { loading: false, testingId: null },
)

const emit = defineEmits<{
  toggle: [row: BackupAccountDto, enabled: boolean]
  test: [row: BackupAccountDto]
  edit: [row: BackupAccountDto]
  remove: [row: BackupAccountDto]
}>()

const { t } = useI18n()
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
