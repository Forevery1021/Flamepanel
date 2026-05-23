<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '@/api/client'
import type { CronJob, CronJobLog } from '@/types'

const jobs = ref<CronJob[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const editingJob = ref<CronJob | null>(null)
const logDrawerVisible = ref(false)
const logJobId = ref(0)
const logJobName = ref('')
const logs = ref<CronJobLog[]>([])

const form = ref({
  name: '',
  schedule: '',
  command: '',
  url: '',
})

const scheduleExamples = [
  { label: '每分钟', expr: '* * * * *' },
  { label: '每5分钟', expr: '*/5 * * * *' },
  { label: '每30分钟', expr: '*/30 * * * *' },
  { label: '每小时', expr: '0 * * * *' },
  { label: '每天凌晨2点', expr: '0 2 * * *' },
  { label: '每周一凌晨3点', expr: '0 3 * * 1' },
]

const fetchJobs = async () => {
  loading.value = true
  try {
    const res = await api.get<CronJob[]>('/cron')
    jobs.value = res.data
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const openCreate = () => {
  editingJob.value = null
  form.value = { name: '', schedule: '* * * * *', command: '', url: '' }
  dialogVisible.value = true
}

const openEdit = (job: CronJob) => {
  editingJob.value = job
  form.value = {
    name: job.name,
    schedule: job.schedule,
    command: job.command || '',
    url: job.url || '',
  }
  dialogVisible.value = true
}

const handleSave = async () => {
  if (!form.value.name || !form.value.schedule) {
    ElMessage.warning('请填写任务名称和Cron表达式')
    return
  }
  if (!form.value.command && !form.value.url) {
    ElMessage.warning('请填写要执行的命令或URL')
    return
  }
  try {
    if (editingJob.value) {
      await api.put(`/cron/${editingJob.value.id}`, {
        name: form.value.name,
        schedule: form.value.schedule,
        command: form.value.command || null,
        url: form.value.url || null,
      })
      ElMessage.success('任务已更新')
    } else {
      await api.post('/cron', {
        name: form.value.name,
        schedule: form.value.schedule,
        command: form.value.command || undefined,
        url: form.value.url || undefined,
      })
      ElMessage.success('任务已创建')
    }
    dialogVisible.value = false
    await fetchJobs()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '保存失败')
  }
}

const handleDelete = async (job: CronJob) => {
  try {
    await ElMessageBox.confirm(`确定要删除任务「${job.name}」吗？`, '确认删除', { type: 'warning' })
  } catch {
    return
  }
  try {
    await api.delete(`/cron/${job.id}`)
    ElMessage.success('任务已删除')
    await fetchJobs()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '删除失败')
  }
}

const handleToggle = async (job: CronJob) => {
  try {
    await api.put(`/cron/${job.id}`, { enabled: !job.enabled })
    job.enabled = !job.enabled
    ElMessage.success(job.enabled ? '任务已启用' : '任务已禁用')
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '操作失败')
  }
}

const handleExecute = async (job: CronJob) => {
  try {
    const res = await api.post(`/cron/${job.id}/execute`)
    ElMessage.success(res.data.message)
    await fetchJobs()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '执行失败')
  }
}

const showLogs = async (job: CronJob) => {
  logJobId.value = job.id
  logJobName.value = job.name
  logDrawerVisible.value = true
  try {
    const res = await api.get<CronJobLog[]>(`/cron/${job.id}/logs?limit=50`)
    logs.value = res.data
  } catch {
    logs.value = []
  }
}

const setSchedule = (expr: string) => {
  form.value.schedule = expr
}

onMounted(() => {
  fetchJobs()
})
</script>

<template>
  <div class="cron-page">
    <div class="page-header">
      <h2>计划任务</h2>
      <p class="desc">管理定时执行的任务，支持 Shell 命令和 URL 请求</p>
    </div>

    <div class="toolbar">
      <el-button type="primary" @click="openCreate">创建任务</el-button>
      <el-button :loading="loading" @click="fetchJobs">刷新列表</el-button>
    </div>

    <!-- Job table -->
    <el-card class="table-card" v-loading="loading">
      <el-table :data="jobs" style="width: 100%" empty-text="暂无计划任务，点击「创建任务」添加">
        <el-table-column prop="name" label="任务名称" min-width="140" />
        <el-table-column prop="schedule" label="Cron 表达式" width="130">
          <template #default="{ row }">
            <el-tag>{{ row.schedule }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="执行内容" min-width="200">
          <template #default="{ row }">
            <span v-if="row.command" class="cmd-text">{{ row.command }}</span>
            <span v-else-if="row.url" class="url-text">{{ row.url }}</span>
            <span v-else style="color: #c0c4cc">-</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-switch
              :model-value="row.enabled"
              @change="handleToggle(row)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="next_run" label="下次运行" width="170" />
        <el-table-column prop="last_run" label="上次运行" width="170">
          <template #default="{ row }">
            <span v-if="row.last_run">{{ row.last_run }}</span>
            <span v-else style="color: #c0c4cc">尚未运行</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" text type="primary" @click="handleExecute(row)">
              立即执行
            </el-button>
            <el-button size="small" text @click="showLogs(row)">
              日志
            </el-button>
            <el-button size="small" text @click="openEdit(row)">
              编辑
            </el-button>
            <el-button size="small" text type="danger" @click="handleDelete(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingJob ? '编辑任务' : '创建任务'"
      width="560px"
      destroy-on-close
    >
      <el-form :model="form" label-width="100px">
        <el-form-item label="任务名称" required>
          <el-input v-model="form.name" placeholder="例如：每日数据库备份" />
        </el-form-item>

        <el-form-item label="Cron 表达式" required>
          <el-input v-model="form.schedule" placeholder="例如：0 2 * * *" />
          <div class="schedule-hints">
            <el-tag
              v-for="ex in scheduleExamples"
              :key="ex.expr"
              size="small"
              style="cursor: pointer; margin: 3px 6px 3px 0"
              :type="form.schedule === ex.expr ? 'primary' : 'info'"
              @click="setSchedule(ex.expr)"
            >
              {{ ex.label }} ({{ ex.expr }})
            </el-tag>
          </div>
        </el-form-item>

        <el-form-item label="执行命令">
          <el-input
            v-model="form.command"
            type="textarea"
            :rows="3"
            placeholder="Shell 命令，例如：tar -czf /backup/site.tar.gz /var/www"
          />
        </el-form-item>

        <el-form-item label="请求 URL">
          <el-input
            v-model="form.url"
            placeholder="HTTP/HTTPS URL，例如：https://example.com/api/cron"
          />
        </el-form-item>
        <el-form-item>
          <span class="form-hint">「执行命令」和「请求 URL」至少填写一项</span>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>

    <!-- Logs Drawer -->
    <el-drawer
      v-model="logDrawerVisible"
      :title="`执行日志 - ${logJobName}`"
      size="480px"
    >
      <div v-if="logs.length === 0" style="color: #909399; text-align: center; padding: 40px 0">
        暂无执行记录
      </div>
      <div v-for="log in logs" :key="log.id" class="log-entry">
        <div class="log-header">
          <el-tag :type="log.status === 'success' ? 'success' : 'danger'" size="small">
            {{ log.status === 'success' ? '成功' : '失败' }}
          </el-tag>
          <span class="log-time">{{ log.started_at }}</span>
        </div>
        <pre v-if="log.output" class="log-output">{{ log.output }}</pre>
      </div>
    </el-drawer>
  </div>
</template>

<style scoped>
.cron-page {
  padding: 24px;
  max-width: 1200px;
}

.page-header h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-primary);
}
.page-header .desc {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 20px 0;
}

.table-card {
  background: var(--bg-card);
  border-color: var(--border-color);
}

.cmd-text {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  color: var(--text-primary);
  background: var(--bg-page);
  padding: 2px 8px;
  border-radius: 4px;
}
.url-text {
  font-size: 12px;
  color: #409eff;
}

.schedule-hints {
  margin-top: 6px;
  line-height: 1.8;
}

.form-hint {
  font-size: 12px;
  color: var(--text-secondary);
}

.log-entry {
  border-bottom: 1px solid var(--border-light);
  padding: 12px 0;
}
.log-entry:last-child {
  border-bottom: none;
}
.log-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.log-time {
  font-size: 12px;
  color: var(--text-secondary);
}
.log-output {
  background: var(--bg-page);
  color: var(--text-primary);
  padding: 10px;
  border-radius: 6px;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow: auto;
  margin: 0;
}
</style>
