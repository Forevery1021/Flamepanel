<template>
  <div class="panel">
    <FpTable :rows="rows" :loading="loading" :paginator="false" :empty-text="t('common.noData')" :empty-desc="t('pageDesc.emptyHint')">
      <template v-if="$slots.emptyAction" #emptyAction><slot name="emptyAction" /></template>
      <FpColumn field="name" :header="t('backup.jobName')" style="min-width: 150px" />
      <FpColumn :header="t('backup.jobScope')" style="width: 130px">
        <template #body="{ data }">
          <FpTag severity="info" :value="jobScopeLabel(data, t)" />
        </template>
      </FpColumn>
      <FpColumn field="schedule" :header="t('backup.jobSchedule')" style="width: 130px">
        <template #body="{ data }">
          <code class="mono-cell">{{ data.schedule }}</code>
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.jobTarget')" style="width: 140px">
        <template #body="{ data }">
          {{ accountNameOf(accounts, data.target_account_id) }}
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.jobRetention')" style="width: 90px">
        <template #body="{ data }">{{ data.retention }}</template>
      </FpColumn>
      <FpColumn :header="t('backup.jobEnabled')" style="width: 90px">
        <template #body="{ data }">
          <FpSwitch
            :model-value="data.enabled"
            @update:model-value="(v: boolean) => emit('toggle', data, v)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.jobLastStatus')" style="width: 110px">
        <template #body="{ data }">
          <FpTag
            :severity="jobStatusTag(data.last_status)"
            :value="jobStatusLabel(data.last_status, t)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('backup.jobNextRun')" style="width: 170px">
        <template #body="{ data }">
          <span>{{ formatDateTime(data.next_run_at) }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.colActions')" style="width: 240px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'backup:update', mode: 'view' }"
              variant="primary"
              :loading="runningId === data.id"
              @click="emit('run', data)"
            >
              {{ t('backup.jobRun') }}
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
import type { BackupAccountDto, BackupJobDto } from '@/api/generated'
import {
  accountNameOf,
  formatDateTime,
  jobScopeLabel,
  jobStatusLabel,
  jobStatusTag,
} from '@/utils/backupFormat'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'

/**
 * 计划备份任务列表（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 本表是三个页签里唯一**跨表引用**的一屏：`target_account_id` 要经账号缓存
 * 译成名字（`accountNameOf`，账号被删则回落 `#id`）。因此账号列表作为 prop
 * 传入而不是在组件内自行拉取——两张表的加载时机由父级统管，否则会出现
 * 「任务表先到、账号表未到」时的一列 `#id`（视觉抖动）。
 *
 * `runningId` 只锁当前行的「立即执行」按钮（原实现如此）。
 */
withDefaults(
  defineProps<{
    rows: BackupJobDto[]
    loading?: boolean
    /** 账号缓存：把 target_account_id 译成名字（跨表引用的唯一数据源） */
    accounts?: BackupAccountDto[]
    /** 正在执行的行 id（只锁该行） */
    runningId?: number | null
  }>(),
  { loading: false, accounts: () => [], runningId: null },
)

const emit = defineEmits<{
  toggle: [row: BackupJobDto, enabled: boolean]
  run: [row: BackupJobDto]
  edit: [row: BackupJobDto]
  remove: [row: BackupJobDto]
}>()

const { t } = useI18n()
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.mono-cell {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
</style>
