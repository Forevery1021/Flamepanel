<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.scheduledTasks') }}</h2>
      <el-button type="primary" @click="openCreate">{{ t('scheduledTask.create') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :empty-text="t('common.noData')"
        :data="tasks"
        border
        stripe
        max-height="620px"
      >
        <el-table-column prop="name" :label="t('scheduledTask.name')" width="160" />
        <el-table-column prop="command" :label="t('scheduledTask.command')" min-width="220">
          <template #default="{ row }">
            <code class="command-cell">{{ row.command }}</code>
          </template>
        </el-table-column>
        <el-table-column prop="schedule" :label="t('scheduledTask.schedule')" width="130">
          <template #default="{ row }">
            <el-tag size="small" type="info">{{ row.schedule }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('scheduledTask.enabled')" width="80">
          <template #default="{ row }">
            <el-switch
              :model-value="row.enabled"
              @change="(val: string | number | boolean) => onToggle(row, Boolean(val))"
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('scheduledTask.lastStatus')" width="100">
          <template #default="{ row }">
            <el-tag size="small" :type="statusTag(row.last_status)">
              {{ statusLabel(row.last_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('scheduledTask.nextRun')" width="160">
          <template #default="{ row }">{{ row.next_run_at || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('scheduledTask.lastRun')" width="160">
          <template #default="{ row }">{{ row.last_run_at || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('common.colActions')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" type="primary" plain @click="onRun(row)">
              {{ t('scheduledTask.run') }}
            </el-button>
            <el-button size="small" @click="openEdit(row)">{{ t('common.edit') }}</el-button>
            <el-button size="small" type="danger" @click="onDelete(row)">
              {{ t('common.delete') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        background
        small
        class="table-pagination"
        @current-change="fetch"
      />
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? t('scheduledTask.edit') : t('scheduledTask.create')"
      width="520px"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item :label="t('scheduledTask.name')" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('scheduledTask.command')" prop="command">
          <el-input v-model="form.command" type="textarea" :rows="3" />
        </el-form-item>
        <el-form-item :label="t('scheduledTask.schedule')" prop="schedule">
          <el-input v-model="form.schedule" placeholder="*/5 * * * *" />
        </el-form-item>
        <el-form-item :label="t('scheduledTask.enabled')">
          <el-switch v-model="form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSave">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="outputVisible" :title="t('scheduledTask.lastOutput')" width="560px">
      <pre class="output-box">{{ selectedOutput || '-' }}</pre>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listScheduledTasks,
  createScheduledTask,
  updateScheduledTask,
  deleteScheduledTask,
  runScheduledTask,
  toggleScheduledTask,
} from '@/api/scheduledTasks'
import type { ScheduledTask } from '@/api/scheduledTasks'

const { t } = useI18n()
const tasks = ref<ScheduledTask[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const dialogVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)
const formRef = ref<FormInstance>()
const outputVisible = ref(false)
const selectedOutput = ref('')

const form = reactive({ name: '', command: '', schedule: '* * * * *', enabled: true })
const rules: FormRules = {
  name: [{ required: true, message: t('scheduledTask.nameRequired'), trigger: 'blur' }],
  command: [{ required: true, message: t('scheduledTask.commandRequired'), trigger: 'blur' }],
  schedule: [{ required: true, message: t('scheduledTask.scheduleRequired'), trigger: 'blur' }],
}

function statusTag(status: string) {
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

function openCreate() {
  editingId.value = 0
  form.name = ''
  form.command = ''
  form.schedule = '* * * * *'
  form.enabled = true
  dialogVisible.value = true
}

function openEdit(row: ScheduledTask) {
  editingId.value = row.id
  form.name = row.name
  form.command = row.command
  form.schedule = row.schedule
  form.enabled = row.enabled
  dialogVisible.value = true
}

async function handleSave() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
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
    ElMessage.success(t('common.success'))
    dialogVisible.value = false
    await fetch()
  } catch (e: unknown) {
    const msg = (e as { message?: string })?.message
    ElMessage.error(msg || t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function onRun(row: ScheduledTask) {
  try {
    await ElMessageBox.confirm(
      t('scheduledTask.runConfirm', { name: row.name }),
      t('common.confirm'),
      { type: 'warning' },
    )
    const res = await runScheduledTask(row.id)
    selectedOutput.value = res.data.last_output
    outputVisible.value = true
    await fetch()
  } catch {
    // cancelled
  }
}

async function onToggle(row: ScheduledTask, enabled: boolean) {
  try {
    await toggleScheduledTask(row.id, enabled)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
    await fetch()
  }
}

async function onDelete(row: ScheduledTask) {
  try {
    await ElMessageBox.confirm(
      t('scheduledTask.deleteConfirm', { name: row.name }),
      t('common.confirm'),
      { type: 'warning' },
    )
    await deleteScheduledTask(row.id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    // cancelled
  }
}

onMounted(fetch)
</script>

<style scoped>
.command-cell {
  font-family: var(--font-mono, monospace);
  font-size: 12px;
  background: var(--bg-hover, #f5f5f5);
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
  border-radius: var(--radius-sm);
}
</style>
