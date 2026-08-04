<template>
  <LayoutContent :title="t('scheduledTask.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('scheduledTask.create') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable
        :rows="tasks"
        :loading="loading"
        :first="(currentPage - 1) * pageSize"
        :empty-text="t('common.noData')"
      >
        <Column field="name" :header="t('scheduledTask.name')" style="width: 160px" />
        <Column :header="t('scheduledTask.command')">
          <template #body="{ data }">
            <code class="command-cell">{{ data.command }}</code>
          </template>
        </Column>
        <Column :header="t('scheduledTask.schedule')" style="width: 130px">
          <template #body="{ data }">
            <FpTag severity="info" :value="data.schedule" />
          </template>
        </Column>
        <Column :header="t('scheduledTask.enabled')" style="width: 80px">
          <template #body="{ data }">
            <ToggleSwitch
              :model-value="data.enabled"
              @update:model-value="(v: boolean) => onToggle(data, v)"
            />
          </template>
        </Column>
        <Column :header="t('scheduledTask.lastStatus')" style="width: 100px">
          <template #body="{ data }">
            <FpTag :severity="statusTag(data.last_status)" :value="statusLabel(data.last_status)" />
          </template>
        </Column>
        <Column :header="t('scheduledTask.nextRun')" style="width: 160px">
          <template #body="{ data }">{{ data.next_run_at || '-' }}</template>
        </Column>
        <Column :header="t('scheduledTask.lastRun')" style="width: 160px">
          <template #body="{ data }">{{ data.last_run_at || '-' }}</template>
        </Column>
        <Column :header="t('common.colActions')" style="width: 200px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="ghost" icon="oi oi-play-circle" @click="onRun(data)">
                {{ t('scheduledTask.run') }}
              </FpButton>
              <FpButton variant="link" @click="openEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton variant="link" @click="onDelete(data)">{{ t('common.delete') }}</FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
      <Paginator
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total-records="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => goPage(f)"
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
          <Textarea
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
          <ToggleSwitch v-model="form.enabled" />
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
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
import Textarea from 'openvue/textarea'
import ToggleSwitch from 'openvue/toggleswitch'
import {
  listScheduledTasks,
  createScheduledTask,
  updateScheduledTask,
  deleteScheduledTask,
  runScheduledTask,
  toggleScheduledTask,
} from '@/api/scheduledTasks'
import type { ScheduledTask } from '@/api/scheduledTasks'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

type TagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const tasks = ref<ScheduledTask[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
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

async function fetch() {
  loading.value = true
  try {
    const res = await listScheduledTasks(currentPage.value, pageSize.value)
    tasks.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
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
    await fetch()
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
        await fetch()
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
    await fetch()
  } catch {
    toast.error(t('common.failed'))
    await fetch()
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
        await fetch()
      } catch {
        // cancelled
      }
    },
  })
}

onMounted(fetch)
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
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
  background: #0f172a;
  color: #e2e8f0;
  padding: 12px;
  border-radius: var(--fp-radius-sm);
}
</style>
