<template>
  <FpModal v-model="visible" :header="t('backup.cloudRestore')" size="lg">
    <div class="modal-form">
      <FpSelect
        v-model="accountId"
        :label="t('backup.jobTarget')"
        :options="accountOptions"
        option-label="label"
        option-value="value"
      />
      <div class="field-row">
        <FpButton variant="primary" :loading="loading" @click="emit('load')">
          {{ t('backup.cloudRestoreLoad') }}
        </FpButton>
        <small class="field-hint">{{ t('backup.cloudRestoreHint') }}</small>
      </div>
      <FpTable
        :rows="objects"
        :loading="loading"
        :paginator="false"
        :empty-text="t('common.noData')"
      >
        <FpColumn :header="t('backup.cloudRestoreKey')" style="min-width: 280px">
          <template #body="{ data }">
            <code class="mono-cell">{{ data.key }}</code>
          </template>
        </FpColumn>
        <FpColumn :header="t('backup.jobScope')" style="width: 110px">
          <template #body="{ data }">
            <FpTag severity="info" :value="restoreScopeLabel(data.key, t)" />
          </template>
        </FpColumn>
        <FpColumn :header="t('backup.size')" style="width: 100px">
          <template #body="{ data }">{{ formatSize(data.size) }}</template>
        </FpColumn>
        <FpColumn :header="t('common.colActions')" style="width: 110px" frozen>
          <template #body="{ data }">
            <FpButton
              v-permission="{ perm: 'backup:create', mode: 'view' }"
              variant="warning"
              :disabled="!isRestorableKey(data.key)"
              :loading="restoringKey === data.key"
              @click="emit('restore', data.key)"
            >
              {{ t('backup.restore') }}
            </FpButton>
          </template>
        </FpColumn>
      </FpTable>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.close') }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteObjectDto } from '@/api/generated'
import { formatSize } from '@/utils/format'
import { isRestorableKey, restoreScopeLabel } from '@/utils/backupFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * 云端恢复弹窗（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 四列（对象 key / 范围 / 体积 / 操作），其中两条是**不可还原对象的可见性**：
 * 远端桶里可能有任意手工上传的文件，它们照样列出（用户需要看到桶里有什么），
 * 但还原按钮禁用（`isRestorableKey`）。若把不可还原的行直接过滤掉，用户会
 * 以为「备份丢了」。
 *
 * 账号选择与对象清单都归父级数据层（`useBackupAccounts`），本组件只做
 * 「选账号 → 上抛 load」「点行 → 上抛 restore」两条线；`restoringKey` 由父级
 * 传入以锁单行（长操作不能重复点）。
 */
withDefaults(
  defineProps<{
    accountOptions: { label: string; value: number }[]
    objects: RemoteObjectDto[]
    loading?: boolean
    restoringKey?: string | null
  }>(),
  { loading: false, restoringKey: null },
)

const emit = defineEmits<{
  load: []
  restore: [key: string]
  'update:accountId': [accountId: number]
}>()

const { t } = useI18n()

const visible = ref(false)
/** 会话级选中账号：父级在打开时下发初值（启用账号首项），此后本身双向同步 */
const accountId = defineModel<number>('accountId', { default: 0 })

/** 打开弹窗（父级在打开后自行决定是否立即拉对象） */
function open() {
  visible.value = true
}

function close() {
  visible.value = false
}

defineExpose({ open, close })
</script>

<style scoped>
.mono-cell {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
</style>
