<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Refresh } from '@element-plus/icons-vue'
import type {
  NotificationChannel, AlertRule, AlertHistory,
  CreateNotificationChannelRequest, CreateAlertRuleRequest,
  UpdateNotificationChannelRequest, UpdateAlertRuleRequest,
} from '@/types'

const API = '/api/alerts'
const activeTab = ref('channels')

// ── Channels ───────────────────────────────────────────────────────────────────

const channels = ref<NotificationChannel[]>([])
const channelsLoading = ref(false)
const channelDialog = ref(false)
const channelDialogTitle = ref('新建通知渠道')
const editingChannelId = ref<number | null>(null)
const channelForm = ref<CreateNotificationChannelRequest>({
  name: '',
  channel_type: 'webhook',
  config: { url: '' },
})
const channelConfigText = ref('')

async function loadChannels() {
  channelsLoading.value = true
  try {
    const resp = await fetch(`${API}/channels`)
    if (!resp.ok) throw new Error('加载失败')
    channels.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载通知渠道失败: ' + e.message)
  } finally {
    channelsLoading.value = false
  }
}

function openCreateChannel() {
  channelDialogTitle.value = '新建通知渠道'
  editingChannelId.value = null
  channelForm.value = { name: '', channel_type: 'webhook', config: { url: '' } }
  channelConfigText.value = JSON.stringify({ url: '' }, null, 2)
  channelDialog.value = true
}

function openEditChannel(channel: NotificationChannel) {
  channelDialogTitle.value = '编辑通知渠道'
  editingChannelId.value = channel.id
  channelForm.value = {
    name: channel.name,
    channel_type: channel.channel_type,
    config: JSON.parse(channel.config || '{}'),
  }
  channelConfigText.value = channel.config
  channelDialog.value = true
}

function onChannelTypeChange() {
  const type = channelForm.value.channel_type
  switch (type) {
    case 'webhook':
      channelConfigText.value = JSON.stringify({ url: '', method: 'POST' }, null, 2)
      break
    case 'email':
      channelConfigText.value = JSON.stringify({ smtp_host: '', smtp_port: 587, username: '', password: '', to: '' }, null, 2)
      break
    case 'telegram':
      channelConfigText.value = JSON.stringify({ bot_token: '', chat_id: '' }, null, 2)
      break
  }
}

async function saveChannel() {
  if (!channelForm.value.name) {
    ElMessage.warning('请填写名称')
    return
  }
  try {
    let config: any
    try {
      config = JSON.parse(channelConfigText.value)
    } catch {
      ElMessage.error('配置 JSON 格式错误')
      return
    }
    const body = { ...channelForm.value, config }

    let resp: Response
    if (editingChannelId.value) {
      resp = await fetch(`${API}/channels/${editingChannelId.value}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
    } else {
      resp = await fetch(`${API}/channels`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
    }
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success(editingChannelId.value ? '更新成功' : '创建成功')
    channelDialog.value = false
    loadChannels()
  } catch (e: any) {
    ElMessage.error('保存失败: ' + e.message)
  }
}

async function deleteChannel(channel: NotificationChannel) {
  try {
    await ElMessageBox.confirm(`确定删除通知渠道 "${channel.name}"？`, '确认删除', { type: 'warning' })
  } catch { return }
  try {
    const resp = await fetch(`${API}/channels/${channel.id}`, { method: 'DELETE' })
    if (!resp.ok) throw new Error('删除失败')
    ElMessage.success('删除成功')
    loadChannels()
  } catch (e: any) {
    ElMessage.error('删除失败: ' + e.message)
  }
}

async function testChannel(channel: NotificationChannel) {
  try {
    const resp = await fetch(`${API}/channels/${channel.id}/test`, { method: 'POST' })
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success('测试消息发送成功')
  } catch (e: any) {
    ElMessage.error('测试失败: ' + e.message)
  }
}

// ── Rules ──────────────────────────────────────────────────────────────────────

const rules = ref<AlertRule[]>([])
const rulesLoading = ref(false)
const ruleDialog = ref(false)
const ruleDialogTitle = ref('新建告警规则')
const editingRuleId = ref<number | null>(null)
const ruleForm = ref<CreateAlertRuleRequest>({
  name: '',
  metric_type: 'cpu',
  condition: 'gt',
  threshold: 90,
  duration_seconds: 60,
  channel_ids: [],
  cooldown_minutes: 5,
})

async function loadRules() {
  rulesLoading.value = true
  try {
    const resp = await fetch(`${API}/rules`)
    if (!resp.ok) throw new Error('加载失败')
    rules.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载告警规则失败: ' + e.message)
  } finally {
    rulesLoading.value = false
  }
}

function openCreateRule() {
  ruleDialogTitle.value = '新建告警规则'
  editingRuleId.value = null
  ruleForm.value = {
    name: '', metric_type: 'cpu', condition: 'gt', threshold: 90,
    duration_seconds: 60, channel_ids: [], cooldown_minutes: 5,
  }
  ruleDialog.value = true
}

function openEditRule(rule: AlertRule) {
  ruleDialogTitle.value = '编辑告警规则'
  editingRuleId.value = rule.id
  ruleForm.value = {
    name: rule.name,
    metric_type: rule.metric_type,
    condition: rule.condition,
    threshold: rule.threshold,
    duration_seconds: rule.duration_seconds,
    channel_ids: JSON.parse(rule.channel_ids || '[]'),
    cooldown_minutes: rule.cooldown_minutes,
  }
  ruleDialog.value = true
}

const enabledChannels = computed(() => channels.value.filter(c => c.enabled))

async function saveRule() {
  if (!ruleForm.value.name) {
    ElMessage.warning('请填写规则名称')
    return
  }
  try {
    let resp: Response
    if (editingRuleId.value) {
      resp = await fetch(`${API}/rules/${editingRuleId.value}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(ruleForm.value),
      })
    } else {
      resp = await fetch(`${API}/rules`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(ruleForm.value),
      })
    }
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success(editingRuleId.value ? '更新成功' : '创建成功')
    ruleDialog.value = false
    loadRules()
  } catch (e: any) {
    ElMessage.error('保存失败: ' + e.message)
  }
}

async function deleteRule(rule: AlertRule) {
  try {
    await ElMessageBox.confirm(`确定删除告警规则 "${rule.name}"？`, '确认删除', { type: 'warning' })
  } catch { return }
  try {
    const resp = await fetch(`${API}/rules/${rule.id}`, { method: 'DELETE' })
    if (!resp.ok) throw new Error('删除失败')
    ElMessage.success('删除成功')
    loadRules()
  } catch (e: any) {
    ElMessage.error('删除失败: ' + e.message)
  }
}

function metricLabel(type: string) {
  switch (type) {
    case 'cpu': return 'CPU 使用率'
    case 'memory': return '内存使用率'
    case 'disk': return '磁盘使用率'
    case 'load': return '系统负载'
    default: return type
  }
}

function conditionLabel(cond: string) {
  switch (cond) {
    case 'gt': return '>'
    case 'lt': return '<'
    case 'gte': return '>='
    case 'lte': return '<='
    case 'eq': return '='
    default: return cond
  }
}

function getChannelNames(channelIdsJson: string): string {
  try {
    const ids: number[] = JSON.parse(channelIdsJson)
    return ids.map(id => channels.value.find(c => c.id === id)?.name || `#${id}`).join(', ') || '-'
  } catch { return '-' }
}

// ── History ────────────────────────────────────────────────────────────────────

const history = ref<AlertHistory[]>([])
const historyLoading = ref(false)

async function loadHistory() {
  historyLoading.value = true
  try {
    const resp = await fetch(`${API}/history?limit=100`)
    if (!resp.ok) throw new Error('加载失败')
    history.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载告警历史失败: ' + e.message)
  } finally {
    historyLoading.value = false
  }
}

function statusTag(status: string) {
  return status === 'firing' ? 'danger' : 'success'
}

function statusText(status: string) {
  return status === 'firing' ? '触发中' : '已恢复'
}

function tabChange(val: string) {
  if (val === 'channels') loadChannels()
  else if (val === 'rules') loadRules()
  else if (val === 'history') loadHistory()
}

onMounted(() => loadChannels())
</script>

<template>
  <div class="alerts-view">
    <div class="page-header">
      <h2>告警管理</h2>
    </div>

    <el-tabs v-model="activeTab" @tab-change="tabChange">
      <!-- Channels Tab -->
      <el-tab-pane label="通知渠道" name="channels">
        <div class="tab-toolbar">
          <el-button type="primary" size="small" :icon="Plus" @click="openCreateChannel">新建渠道</el-button>
          <el-button size="small" :icon="Refresh" @click="loadChannels" :loading="channelsLoading" />
        </div>
        <div class="card-grid">
          <div v-for="c in channels" :key="c.id" class="channel-card">
            <div class="card-header">
              <span class="card-name">{{ c.name }}</span>
              <el-tag :type="c.enabled ? 'success' : 'info'" size="small">{{ c.enabled ? '启用' : '禁用' }}</el-tag>
            </div>
            <div class="card-meta">
              <span class="channel-type-tag">{{ c.channel_type.toUpperCase() }}</span>
              <span class="card-date">{{ c.updated_at }}</span>
            </div>
            <div class="card-actions">
              <el-button size="small" text type="success" @click="testChannel(c)">测试</el-button>
              <el-button size="small" text @click="openEditChannel(c)">编辑</el-button>
              <el-button size="small" text type="danger" @click="deleteChannel(c)">删除</el-button>
            </div>
          </div>
          <el-empty v-if="!channelsLoading && channels.length === 0" description="暂无通知渠道" />
        </div>
      </el-tab-pane>

      <!-- Rules Tab -->
      <el-tab-pane label="告警规则" name="rules">
        <div class="tab-toolbar">
          <el-button type="primary" size="small" :icon="Plus" @click="openCreateRule">新建规则</el-button>
          <el-button size="small" :icon="Refresh" @click="loadRules" :loading="rulesLoading" />
        </div>
        <div class="card-grid">
          <div v-for="r in rules" :key="r.id" class="rule-card">
            <div class="card-header">
              <span class="card-name">{{ r.name }}</span>
              <el-tag :type="r.enabled ? 'success' : 'info'" size="small">{{ r.enabled ? '启用' : '禁用' }}</el-tag>
            </div>
            <div class="rule-condition">
              {{ metricLabel(r.metric_type) }} <b>{{ conditionLabel(r.condition) }} {{ r.threshold }}</b>
            </div>
            <div class="card-meta">
              <span>持续 {{ r.duration_seconds }}s | 冷却 {{ r.cooldown_minutes }}min</span>
              <span class="meta-channels">通知: {{ getChannelNames(r.channel_ids) }}</span>
            </div>
            <div v-if="r.last_triggered" class="last-trigger">最近触发: {{ r.last_triggered }}</div>
            <div class="card-actions">
              <el-button size="small" text @click="openEditRule(r)">编辑</el-button>
              <el-button size="small" text type="danger" @click="deleteRule(r)">删除</el-button>
            </div>
          </div>
          <el-empty v-if="!rulesLoading && rules.length === 0" description="暂无告警规则" />
        </div>
      </el-tab-pane>

      <!-- History Tab -->
      <el-tab-pane label="告警历史" name="history">
        <div class="tab-toolbar">
          <el-button size="small" :icon="Refresh" @click="loadHistory" :loading="historyLoading" />
        </div>
        <el-table :data="history" stripe size="small" max-height="calc(100vh - 280px)">
          <el-table-column prop="rule_name" label="规则" min-width="120" />
          <el-table-column prop="metric_type" label="指标" width="80">
            <template #default="{ row }">{{ metricLabel(row.metric_type) }}</template>
          </el-table-column>
          <el-table-column prop="metric_value" label="当前值" width="80" />
          <el-table-column prop="threshold" label="阈值" width="80" />
          <el-table-column prop="status" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="statusTag(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="message" label="消息" min-width="200" show-overflow-tooltip />
          <el-table-column prop="created_at" label="时间" width="160" />
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <!-- Channel Dialog -->
    <el-dialog v-model="channelDialog" :title="channelDialogTitle" width="560px">
      <el-form :model="channelForm" label-width="100px">
        <el-form-item label="名称" required>
          <el-input v-model="channelForm.name" placeholder="渠道名称" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="channelForm.channel_type" style="width: 100%" @change="onChannelTypeChange">
            <el-option label="Webhook" value="webhook" />
            <el-option label="邮件 (SMTP)" value="email" />
            <el-option label="Telegram Bot" value="telegram" />
          </el-select>
        </el-form-item>
        <el-form-item label="配置 (JSON)">
          <el-input
            v-model="channelConfigText"
            type="textarea"
            :rows="8"
            placeholder='{"url": "https://..."}'
            style="font-family: monospace; font-size: 13px"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="channelDialog = false">取消</el-button>
        <el-button type="primary" @click="saveChannel">保存</el-button>
      </template>
    </el-dialog>

    <!-- Rule Dialog -->
    <el-dialog v-model="ruleDialog" :title="ruleDialogTitle" width="520px">
      <el-form :model="ruleForm" label-width="100px">
        <el-form-item label="规则名称" required>
          <el-input v-model="ruleForm.name" placeholder="CPU 过高告警" />
        </el-form-item>
        <el-form-item label="监控指标">
          <el-select v-model="ruleForm.metric_type" style="width: 100%">
            <el-option label="CPU 使用率" value="cpu" />
            <el-option label="内存使用率" value="memory" />
            <el-option label="磁盘使用率" value="disk" />
            <el-option label="系统负载" value="load" />
          </el-select>
        </el-form-item>
        <el-form-item label="条件">
          <el-row :gutter="12">
            <el-col :span="8">
              <el-select v-model="ruleForm.condition" style="width: 100%">
                <el-option label="大于 (>)" value="gt" />
                <el-option label="小于 (<)" value="lt" />
                <el-option label="大于等于 (>=)" value="gte" />
                <el-option label="小于等于 (<=)" value="lte" />
                <el-option label="等于 (=)" value="eq" />
              </el-select>
            </el-col>
            <el-col :span="16">
              <el-input-number v-model="ruleForm.threshold" :min="0" :max="100" style="width: 100%" />
            </el-col>
          </el-row>
        </el-form-item>
        <el-form-item label="持续时间(s)">
          <el-input-number v-model="ruleForm.duration_seconds" :min="0" :max="3600" />
        </el-form-item>
        <el-form-item label="冷却(min)">
          <el-input-number v-model="ruleForm.cooldown_minutes" :min="1" :max="1440" />
        </el-form-item>
        <el-form-item label="通知渠道">
          <el-select v-model="ruleForm.channel_ids" multiple placeholder="选择通知渠道" style="width: 100%">
            <el-option
              v-for="c in enabledChannels"
              :key="c.id"
              :label="`${c.name} (${c.channel_type})`"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ruleDialog = false">取消</el-button>
        <el-button type="primary" @click="saveRule">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.alerts-view {
  padding: 24px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  color: var(--text-primary);
}

.tab-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 14px;
}

.channel-card,
.rule-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.card-name {
  font-weight: 600;
  color: var(--text-primary);
  font-size: 15px;
}

.card-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 10px;
}

.channel-type-tag {
  background: #e8f0fe;
  color: #409eff;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
}

.card-date {
  font-size: 11px;
}

.rule-condition {
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: 8px;
  padding: 8px 12px;
  background: var(--bg-hover);
  border-radius: 6px;
}

.rule-condition b {
  color: #e6a23c;
}

.meta-channels {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.last-trigger {
  font-size: 11px;
  color: #f56c6c;
  margin-bottom: 8px;
}

.card-actions {
  display: flex;
  gap: 4px;
}
</style>
