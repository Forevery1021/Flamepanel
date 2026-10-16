<template>
  <div class="panel">
    <FpInlineMessage severity="info">{{ t('aiCenter.approvals.hint') }}</FpInlineMessage>
    <div class="filters">
      <FpSelect
        v-model="statusFilter"
        :options="statusOptions"
        option-label="label"
        option-value="value"
        :aria-label="t('aiCenter.approvals.filterStatus')"
        class="filter-select"
        @update:model-value="onFilter"
      />
    </div>
    <FpTable :rows="rows" :loading="loading" :empty-text="t('aiCenter.approvals.empty')" striped-rows>
      <FpColumn :header="t('aiCenter.approvals.requestedAt')" style="width: 170px">
        <template #body="{ data }">
          <span class="mono">{{ data.requested_at }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('aiCenter.tool')" style="width: 200px">
        <template #body="{ data }">
          <div class="tool-cell">
            <span class="mono">{{ data.tool }}</span>
            <FpTag :severity="riskSeverity(data.risk)" :value="t(`aiCenter.risks.${data.risk}`)" />
          </div>
        </template>
      </FpColumn>
      <FpColumn :header="t('aiCenter.principal')" style="width: 140px">
        <template #body="{ data }">{{ principalName(data.principal_id) }}</template>
      </FpColumn>
      <FpColumn :header="t('aiCenter.approvals.preview')" style="min-width: 260px">
        <template #body="{ data }">
          <div class="preview">{{ data.preview }}</div>
          <code class="args" :title="data.arguments">{{ data.arguments }}</code>
        </template>
      </FpColumn>
      <FpColumn :header="t('aiCenter.state')" style="width: 110px">
        <template #body="{ data }">
          <FpTag :severity="statusSeverity(data.status)" :value="t(`aiCenter.approvals.statuses.${data.status}`)" />
        </template>
      </FpColumn>
      <FpColumn :header="t('aiCenter.approvals.outcome')" style="min-width: 180px">
        <template #body="{ data }">
          <span v-if="data.status === 'pending'" class="muted mono">
            {{ t('aiCenter.approvals.expiresAt') }} {{ data.expires_at }}
          </span>
          <code v-else-if="data.result || data.reason" class="args" :title="data.result || data.reason">
            {{ data.result || data.reason }}
          </code>
          <span v-else class="muted">—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.operation')" style="width: 150px">
        <template #body="{ data }">
          <div v-if="data.status === 'pending' && canDecide(data)" class="row-actions">
            <FpButton
              size="small"
              :variant="data.risk === 'destructive' ? 'danger' : 'primary'"
              :label="t('aiCenter.approvals.approve')"
              :loading="busyId === data.id"
              @click="confirmApprove(data)"
            />
            <FpButton
              size="small"
              variant="secondary"
              :label="t('aiCenter.approvals.reject')"
              :disabled="busyId === data.id"
              @click="openReject(data)"
            />
          </div>
        </template>
      </FpColumn>
    </FpTable>
    <FpPagination
      v-if="total > pageSize"
      :first="(page - 1) * pageSize"
      :rows="pageSize"
      :total="total"
      :rows-per-page-options="[20, 50, 100]"
      @update:first="onFirst"
    />

    <FpModal v-model="rejectVisible" :header="t('aiCenter.approvals.rejectTitle')" size="md">
      <p v-if="rejecting" class="preview">{{ rejecting.preview }}</p>
      <FpTextarea v-model="rejectReason" :label="t('aiCenter.approvals.rejectReason')" rows="3" maxlength="500" />
      <template #footer>
        <FpButton variant="secondary" :label="t('common.cancel')" @click="rejectVisible = false" />
        <FpButton
          variant="danger"
          :label="t('aiCenter.approvals.reject')"
          :loading="busyId === rejecting?.id"
          @click="submitReject"
        />
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { hasPermission } from '@/directives/permission'
import { useAuthStore } from '@/stores/auth'
import {
  approveApproval,
  listApprovals,
  rejectApproval,
  type ApprovalPage,
  type ToolApproval,
  type ToolInfo,
} from '@/api/mcp'

/**
 * M-AI-2 写操作审批面板（Doc/34 §审批）。
 * 按钮可见性与后端字段级授权一致：mcp:update + 工具本身权限；高危工具另需 mcp:delete。
 */
const props = defineProps<{
  /** 凭证 id → 名称 */
  principalName: (id: number) => string
  /** 工具目录（取工具所需权限） */
  tools: ToolInfo[]
}>()

const emit = defineEmits<{ changed: [] }>()

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const auth = useAuthStore()
const queryClient = useQueryCacheClient()

const STATUSES = ['pending', 'executing', 'succeeded', 'failed', 'rejected', 'expired'] as const
const statusFilter = ref<string>('pending')
const statusOptions = computed(() => [
  { value: '', label: t('aiCenter.approvals.all') },
  ...STATUSES.map((s) => ({ value: s, label: t(`aiCenter.approvals.statuses.${s}`) })),
])
const page = ref(1)
const pageSize = ref(20)

const query = useApiQuery<ApprovalPage>(
  () => queryKeys.mcp.approvals(statusFilter.value, page.value, pageSize.value),
  () =>
    listApprovals({
      status: statusFilter.value || null,
      page: page.value,
      page_size: pageSize.value,
    }),
)
const rows = computed<ToolApproval[]>(() => query.data.value?.data ?? [])
const total = computed(() => query.data.value?.total ?? 0)
const loading = query.loading

function onFilter() {
  page.value = 1
}

function onFirst(first: number) {
  page.value = Math.floor(first / pageSize.value) + 1
}

function riskSeverity(r: string): 'info' | 'warning' | 'danger' {
  return r === 'destructive' ? 'danger' : r === 'write' ? 'warning' : 'info'
}

function statusSeverity(s: string): 'success' | 'danger' | 'warning' | 'info' | 'neutral' {
  switch (s) {
    case 'succeeded':
      return 'success'
    case 'failed':
      return 'danger'
    case 'pending':
    case 'executing':
      return 'warning'
    case 'rejected':
      return 'info'
    default:
      return 'neutral'
  }
}

function canDecide(a: ToolApproval): boolean {
  if (!hasPermission(auth.role, 'mcp:update')) return false
  const perm = props.tools.find((x) => x.name === a.tool)?.permission
  if (perm && !hasPermission(auth.role, perm)) return false
  return a.risk !== 'destructive' || hasPermission(auth.role, 'mcp:delete')
}

const busyId = ref<number | null>(null)

async function refresh() {
  await queryClient.invalidateQueries({ queryKey: queryKeys.mcp.all })
  emit('changed')
}

function confirmApprove(a: ToolApproval) {
  const destructive = a.risk === 'destructive'
  confirmAction({
    header: t('aiCenter.approvals.approve'),
    message: t(destructive ? 'aiCenter.approvals.approveDestructive' : 'aiCenter.approvals.approveConfirm', {
      preview: a.preview,
    }),
    acceptLabel: t('aiCenter.approvals.approve'),
    danger: destructive,
    accept: async () => {
      busyId.value = a.id
      try {
        const { data } = await approveApproval(a.id)
        if (data.status === 'succeeded') toast.success(t('aiCenter.approvals.approved'))
        else toast.error(null, t('aiCenter.approvals.approveFailed', { error: data.result }))
      } catch (e) {
        toast.error(e)
      } finally {
        busyId.value = null
        await refresh()
      }
    },
  })
}

const rejectVisible = ref(false)
const rejecting = ref<ToolApproval | null>(null)
const rejectReason = ref('')

function openReject(a: ToolApproval) {
  rejecting.value = a
  rejectReason.value = ''
  rejectVisible.value = true
}

async function submitReject() {
  const a = rejecting.value
  if (!a) return
  busyId.value = a.id
  try {
    await rejectApproval(a.id, rejectReason.value.trim())
    toast.success(t('aiCenter.approvals.rejected'))
    rejectVisible.value = false
  } catch (e) {
    toast.error(e)
  } finally {
    busyId.value = null
    await refresh()
  }
}
</script>

<style scoped>
.filters {
  display: flex;
  gap: 0.75rem;
  margin: 0.75rem 0;
}
.filter-select {
  width: 200px;
}
.tool-cell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
}
.preview {
  margin-bottom: 0.25rem;
  line-height: 1.5;
}
.args {
  display: block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
  color: var(--fp-text-secondary);
}
.row-actions {
  display: flex;
  gap: 0.375rem;
}
.mono {
  font-family: var(--fp-font-mono, monospace);
  font-size: 0.8125rem;
}
.muted {
  color: var(--fp-text-secondary);
}
</style>
