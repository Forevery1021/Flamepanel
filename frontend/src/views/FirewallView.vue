<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>防火墙管理</h2>
      <div class="actions">
        <el-tag :type="statusTag" size="large" class="status-tag">
          {{ backendInfo }}
        </el-tag>
        <el-button type="primary" @click="showCreateDialog = true">
          添加规则
        </el-button>
        <el-button @click="handleApply">
          应用规则
        </el-button>
        <el-button :type="firewallEnabled ? 'danger' : 'success'" @click="handleToggleFirewall">
          {{ firewallEnabled ? '关闭防火墙' : '开启防火墙' }}
        </el-button>
        <el-button @click="loadData" :loading="loading">
          刷新
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
    <el-table :data="rules" stripe v-loading="loading">
      <el-table-column type="index" label="#" width="50" />
      <el-table-column prop="name" label="名称" min-width="140" />
      <el-table-column label="协议" width="80">
        <template #default="{ row }">
          <el-tag size="small" :type="row.protocol === 'any' ? 'info' : 'primary'">
            {{ row.protocol.toUpperCase() }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="port" label="端口" width="120">
        <template #default="{ row }">
          {{ row.port || '所有' }}
        </template>
      </el-table-column>
      <el-table-column prop="source" label="来源" width="140">
        <template #default="{ row }">
          {{ row.source || '0.0.0.0/0' }}
        </template>
      </el-table-column>
      <el-table-column label="动作" width="90">
        <template #default="{ row }">
          <el-tag size="small" :type="row.action === 'allow' ? 'success' : 'danger'">
            {{ row.action === 'allow' ? '允许' : row.action === 'deny' ? '拒绝' : '驳回' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="方向" width="80">
        <template #default="{ row }">
          {{ row.direction === 'in' ? '入站' : '出站' }}
        </template>
      </el-table-column>
      <el-table-column label="状态" width="80">
        <template #default="{ row }">
          <el-switch :model-value="row.enabled" @change="(v: boolean) => handleToggle(row.id, v)" />
        </template>
      </el-table-column>
      <el-table-column label="操作" width="160" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="handleEdit(row)">编辑</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row.id)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
    </el-card>

    <el-dialog v-model="showCreateDialog" title="添加防火墙规则" width="560px">
      <el-form :model="form" label-width="80px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="例如：允许 SSH" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" placeholder="规则描述（可选）" />
        </el-form-item>
        <el-form-item label="协议" required>
          <el-select v-model="form.protocol" style="width: 100%">
            <el-option label="TCP" value="tcp" />
            <el-option label="UDP" value="udp" />
            <el-option label="ICMP" value="icmp" />
            <el-option label="任意" value="any" />
          </el-select>
        </el-form-item>
        <el-form-item label="端口">
          <el-input v-model="form.port" placeholder="例如：80, 443, 8000-9000（留空=所有）" />
        </el-form-item>
        <el-form-item label="来源 IP">
          <el-input v-model="form.source" placeholder="例如：0.0.0.0/0" />
        </el-form-item>
        <el-form-item label="动作" required>
          <el-select v-model="form.action" style="width: 100%">
            <el-option label="允许 (ALLOW)" value="allow" />
            <el-option label="拒绝 (DENY)" value="deny" />
            <el-option label="驳回 (REJECT)" value="reject" />
          </el-select>
        </el-form-item>
        <el-form-item label="方向" required>
          <el-select v-model="form.direction" style="width: 100%">
            <el-option label="入站" value="in" />
            <el-option label="出站" value="out" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-input-number v-model="form.priority" :min="1" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateDialog = false">取消</el-button>
        <el-button type="primary" @click="handleCreate" :loading="saving">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showEditDialog" title="编辑防火墙规则" width="560px">
      <el-form :model="editForm" label-width="80px">
        <el-form-item label="名称" required>
          <el-input v-model="editForm.name" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="editForm.description" />
        </el-form-item>
        <el-form-item label="协议" required>
          <el-select v-model="editForm.protocol" style="width: 100%">
            <el-option label="TCP" value="tcp" />
            <el-option label="UDP" value="udp" />
            <el-option label="ICMP" value="icmp" />
            <el-option label="任意" value="any" />
          </el-select>
        </el-form-item>
        <el-form-item label="端口">
          <el-input v-model="editForm.port" placeholder="留空=所有" />
        </el-form-item>
        <el-form-item label="来源 IP">
          <el-input v-model="editForm.source" />
        </el-form-item>
        <el-form-item label="动作" required>
          <el-select v-model="editForm.action" style="width: 100%">
            <el-option label="允许 (ALLOW)" value="allow" />
            <el-option label="拒绝 (DENY)" value="deny" />
            <el-option label="驳回 (REJECT)" value="reject" />
          </el-select>
        </el-form-item>
        <el-form-item label="方向" required>
          <el-select v-model="editForm.direction" style="width: 100%">
            <el-option label="入站" value="in" />
            <el-option label="出站" value="out" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-input-number v-model="editForm.priority" :min="1" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showEditDialog = false">取消</el-button>
        <el-button type="primary" @click="handleSaveEdit" :loading="saving">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listFirewallRules, createFirewallRule, updateFirewallRule, deleteFirewallRule,
  toggleFirewallRule, applyFirewallRules, getFirewallStatus, enableFirewall, disableFirewall,
} from '@/api/firewall'
import type { FirewallRule as FR, FirewallStatus } from '@/types'

const rules = ref<FR[]>([])
const loading = ref(false)
const saving = ref(false)
const showCreateDialog = ref(false)
const showEditDialog = ref(false)
const firewallStatus = ref<FirewallStatus | null>(null)

const form = ref({
  name: '', description: '', protocol: 'tcp', port: '',
  source: '0.0.0.0/0', action: 'allow', direction: 'in', priority: 50,
})

const editForm = ref<FR>({} as FR)

const statusTag = computed(() => {
  if (!firewallStatus.value) return 'info'
  const s = (firewallStatus.value.status || '').toLowerCase()
  if (s.includes('active') || s.includes('running') || s.includes('enable')) return 'success'
  if (s.includes('inactive') || s.includes('disable')) return 'danger'
  return 'warning'
})

const backendInfo = computed(() => {
  if (!firewallStatus.value) return '检测中...'
  const name = firewallStatus.value.backend_name || ''
  const st = firewallStatus.value.status || ''
  return `${name.toUpperCase()} - ${st.substring(0, 40)}`
})

const firewallEnabled = computed(() => {
  if (!firewallStatus.value) return true
  const s = (firewallStatus.value.status || '').toLowerCase()
  return s.includes('active') || s.includes('running') || s.includes('enable')
})

async function loadData() {
  loading.value = true
  try {
    const [ruleRes, statusRes] = await Promise.all([
      listFirewallRules(),
      getFirewallStatus().catch(() => null),
    ])
    rules.value = ruleRes.data
    if (statusRes) firewallStatus.value = statusRes.data
  } catch (e: any) {
    ElMessage.error('加载防火墙规则失败: ' + (e.message || ''))
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  if (!form.value.name) {
    ElMessage.warning('请输入规则名称')
    return
  }
  saving.value = true
  try {
    const data: Record<string, any> = { name: form.value.name, protocol: form.value.protocol, action: form.value.action, direction: form.value.direction, priority: form.value.priority, source: form.value.source || '0.0.0.0/0' }
    if (form.value.description) data.description = form.value.description
    if (form.value.port) data.port = form.value.port
    await createFirewallRule(data)
    ElMessage.success('规则创建成功')
    showCreateDialog.value = false
    form.value = { name: '', description: '', protocol: 'tcp', port: '', source: '0.0.0.0/0', action: 'allow', direction: 'in', priority: 50 }
    await loadData()
  } catch (e: any) {
    ElMessage.error('创建失败: ' + (e.message || ''))
  } finally {
    saving.value = false
  }
}

function handleEdit(rule: FR) {
  editForm.value = { ...rule }
  showEditDialog.value = true
}

async function handleSaveEdit() {
  saving.value = true
  try {
    const data: Record<string, any> = {}
    if (editForm.value.name) data.name = editForm.value.name
    data.description = editForm.value.description
    data.protocol = editForm.value.protocol
    data.port = editForm.value.port
    data.source = editForm.value.source
    data.action = editForm.value.action
    data.direction = editForm.value.direction
    data.priority = editForm.value.priority
    await updateFirewallRule(editForm.value.id, data)
    ElMessage.success('规则更新成功')
    showEditDialog.value = false
    await loadData()
  } catch (e: any) {
    ElMessage.error('更新失败: ' + (e.message || ''))
  } finally {
    saving.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm('确定删除此规则？', '确认')
  } catch {
    return
  }
  try {
    await deleteFirewallRule(id)
    ElMessage.success('规则已删除')
    await loadData()
  } catch (e: any) {
    ElMessage.error('删除失败: ' + (e.message || ''))
  }
}

async function handleToggle(id: number, enabled: boolean) {
  try {
    await toggleFirewallRule(id, enabled)
    ElMessage.success(enabled ? '规则已启用' : '规则已禁用')
    await loadData()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || ''))
  }
}

async function handleApply() {
  try {
    await applyFirewallRules()
    ElMessage.success('防火墙规则已应用')
  } catch (e: any) {
    ElMessage.error('应用失败: ' + (e.message || ''))
  }
}

async function handleToggleFirewall() {
  try {
    if (firewallEnabled.value) {
      await disableFirewall()
      ElMessage.success('防火墙已关闭')
    } else {
      await enableFirewall()
      ElMessage.success('防火墙已开启')
    }
    await loadData()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || ''))
  }
}

onMounted(loadData)
</script>

<style scoped>
.actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.status-tag {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
