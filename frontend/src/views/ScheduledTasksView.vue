<template>
  <LayoutContent :title="t('scheduledTask.title')" reload @reload="invalidate">
    <template #toolbar>
      <!-- P4：统一 PageToolbar -->
      <PageToolbar v-model="searchText">
        <template #left>
          <FpSelect
            v-model="statusFilter"
            :options="statusOptions"
            option-label="label"
            option-value="value"
            show-clear
            class="toolbar-filter"
          />
        </template>
        <template #actions>
          <FpButton v-permission="{ perm: 'scheduled_task:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openCreate">
            {{ t('scheduledTask.create') }}
          </FpButton>
        </template>
      </PageToolbar>
    </template>

    <div class="panel">
      <FpTable
        :rows="filteredTasks"
        :loading="loading"
        :first="first"
        :empty-text="t('common.noData')"
      >
        <FpColumn field="name" :header="t('scheduledTask.name')" style="width: 160px" />
        <FpColumn :header="t('scheduledTask.command')">
          <template #body="{ data }">
            <code class="command-cell">{{ data.command }}</code>
          </template>
        </FpColumn>
        <FpColumn :header="t('scheduledTask.schedule')" style="width: 130px">
          <template #body="{ data }">
            <FpTag severity="info" :value="data.schedule" />
          </template>
        </FpColumn>
        <FpColumn :header="t('scheduledTask.enabled')" style="width: 80px">
          <template #body="{ data }">
            <FpSwitch
              :model-value="data.enabled"
              @update:model-value="(v: boolean) => onToggle(data, v)"
            />
          </template>
        </FpColumn>
        <FpColumn :header="t('scheduledTask.lastStatus')" style="width: 100px">
          <template #body="{ data }">
            <FpTag :severity="statusTag(data.last_status)" :value="statusLabel(data.last_status)" />
          </template>
        </FpColumn>
        <FpColumn :header="t('scheduledTask.nextRun')" style="width: 160px">
          <template #body="{ data }">{{ data.next_run_at || '-' }}</template>
        </FpColumn>
        <FpColumn :header="t('scheduledTask.lastRun')" style="width: 160px">
          <template #body="{ data }">{{ data.last_run_at || '-' }}</template>
        </FpColumn>
        <FpColumn :header="t('common.colActions')" style="width: 200px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton v-permission="{ perm: 'scheduled_task:execute', mode: 'view' }" variant="ghost" icon="oi oi-play-circle" @click="onRun(data)">
                {{ t('scheduledTask.run') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'scheduled_task:update', mode: 'view' }" variant="link" @click="openEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton v-permission="{ perm: 'scheduled_task:delete', mode: 'view' }" variant="link" @click="onDelete(data)">{{ t('common.delete') }}</FpButton>
            </div>
          </template>
        </FpColumn>
      </FpTable>
      <FpPagination
        v-if="total > pageSize"
        :first="first"
        :rows="pageSize"
        :total="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => onFirst(f)"
      />
    </div>

    <FpModal
      v-model="dialogVisible"
      :header="editingId ? t('scheduledTask.edit') : t('scheduledTask.create')"
      style="width: 520px"
    >
      <div class="modal-form">
        <FpInput v-model="form.name" :label="t('scheduledTask.name')" :error="formErrors.name" />
        <div class="field-col">
          <label class="field-label">{{ t('scheduledTask.command') }}</label>
          <FpTextarea
            v-model="form.command"
            :rows="3"
            :invalid="!!formErrors.command"
            class="w-full"
          />
          <small v-if="formErrors.command" class="field-error">{{ formErrors.command }}</small>
        </div>
        <FpInput
          v-model="form.schedule"
          :label="t('scheduledTask.schedule')"
          placeholder="*/5 * * * *"
          :error="formErrors.schedule"
        />
        <div class="field-row">
          <span class="field-label">{{ t('scheduledTask.enabled') }}</span>
          <FpSwitch v-model="form.enabled" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSave">
          {{ t('common.save') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="outputVisible" :header="t('scheduledTask.lastOutput')" style="width: 560px">
      <pre class="output-box">{{ selectedOutput || '-' }}</pre>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'




import {
  listScheduledTasks,
  createScheduledTask,
  updateScheduledTask,
  deleteScheduledTask,
  runScheduledTask,
  toggleScheduledTask,
} from '@/api/scheduledTasks'
import type { ScheduledTask } from '@/api/scheduledTasks'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { useCrudPage } from '@/composables/useCrudPage'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'

type TagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()
// P4：统一 CRUD 分页状态
const crud = useCrudPage()
const { total, first, pageSize, onFirst } = crud

// M9：搜索 + 状态筛选（当前页客户端过滤）
const searchText = ref('')
const statusFilter = ref<string>('')
const statusOptions = computed(() => [
  { label: t('scheduledTask.statusEnabled'), value: 'enabled' },
  { label: t('scheduledTask.statusDisabled'), value: 'disabled' },
  { label: t('scheduledTask.statusSuccess'), value: 'success' },
  { label: t('scheduledTask.statusFailed'), value: 'failed' },
])
const filteredTasks = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return tasks.value.filter((tk) => {
    if (statusFilter.value === 'enabled' && !tk.enabled) return false
    if (statusFilter.value === 'disabled' && tk.enabled) return false
    if (statusFilter.value === 'success' && tk.last_status !== 'success') return false
    if (statusFilter.value === 'failed' && tk.last_status !== 'failed') return false
    if (!kw) return true
    return tk.name.toLowerCase().includes(kw) || tk.command.toLowerCase().includes(kw)
  })
})
const dialogVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)
const outputVisible = ref(false)
const selectedOutput = ref('')

const form = reactive({ name: '', command: '', schedule: '* * * * *', enabled: true })
const formErrors = reactive({ name: '', command: '', schedule: '' })

function statusTag(status: string): TagSeverity {
  if (status === 'success') return 'success'
  if (status === 'failed') return 'danger'
  return 'info'
}

function statusLabel(status: string) {
  if (status === 'success') return t('scheduledTask.statusSuccess')
  if (status === 'failed') return t('scheduledTask.statusFailed')
  return t('scheduledTask.statusNever')
}

// P3-A：列表走统一数据获取层 useApiQuery
const tasksQuery = useApiQuery<{ data: ScheduledTask[]; total: number }>(
  () => queryKeys.scheduledTasks.list(crud.currentPage.value, crud.pageSize.value),
  async () => {
    const res = await listScheduledTasks(crud.currentPage.value, crud.pageSize.value)
    crud.total.value = res.data.total
    return { data: { data: res.data.data, total: res.data.total } }
  },
  { keepPrevious: true },
)
const tasks = computed<ScheduledTask[]>(() => tasksQuery.data.value?.data ?? [])
const loading = tasksQuery.loading

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.scheduledTasks.all })
}

function openCreate() {
  editingId.value = 0
  form.name = ''
  form.command = ''
  form.schedule = '* * * * *'
  form.enabled = true
  formErrors.name = ''
  formErrors.command = ''
  formErrors.schedule = ''
  dialogVisible.value = true
}

function openEdit(row: ScheduledTask) {
  editingId.value = row.id
  form.name = row.name
  form.command = row.command
  form.schedule = row.schedule
  form.enabled = row.enabled
  formErrors.name = ''
  formErrors.command = ''
  formErrors.schedule = ''
  dialogVisible.value = true
}

function validateForm(): boolean {
  formErrors.name = form.name ? '' : t('scheduledTask.nameRequired')
  formErrors.command = form.command ? '' : t('scheduledTask.commandRequired')
  formErrors.schedule = form.schedule ? '' : t('scheduledTask.scheduleRequired')
  return !formErrors.name && !formErrors.command && !formErrors.schedule
}

async function handleSave() {
  if (!validateForm()) return
  submitting.value = true
  try {
    if (editingId.value) {
      await updateScheduledTask(editingId.value, {
        name: form.name,
        command: form.command,
        schedule: form.schedule,
        enabled: form.enabled,
      })
    } else {
      await createScheduledTask({
        name: form.name,
        command: form.command,
        schedule: form.schedule,
        enabled: form.enabled,
      })
    }
    toast.success(t('common.success'))
    dialogVisible.value = false
    invalidate()
  } catch (e: unknown) {
    toast.error(e, t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function onRun(row: ScheduledTask) {
  confirmAction({
    message: t('scheduledTask.runConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        const res = await runScheduledTask(row.id)
        selectedOutput.value = res.data.last_output
        outputVisible.value = true
        invalidate()
      } catch {
        // cancelled
      }
    },
  })
}

async function onToggle(row: ScheduledTask, enabled: boolean) {
  try {
    await toggleScheduledTask(row.id, enabled)
    toast.success(t('common.success'))
    invalidate()
  } catch {
    toast.error(t('common.failed'))
    invalidate()
  }
}

function onDelete(row: ScheduledTask) {
  confirmAction({
    message: t('scheduledTask.deleteConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await deleteScheduledTask(row.id)
        toast.success(t('common.success'))
        invalidate()
      } catch {
        // cancelled
      }
    },
  })
}
</script>

<style scoped>
.toolbar-filter {
  width: 160px;
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
.field-col {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-row {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
}
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.field-error {
  font-size: 12px;
  line-height: 1.4;
  color: var(--fp-danger);
}
.command-cell {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  background: var(--fp-bg-hover);
  padding: 2px 6px;
  border-radius: 4px;
}
.output-box {
  max-height: 400px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 12px;
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
  padding: 12px;
  border-radius: var(--fp-radius-sm);
}
</style>
