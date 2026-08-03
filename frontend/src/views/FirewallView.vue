<template>
  <div class="view-container">
    <div class="card-header-title">
      <div class="actions">
        <el-tag :type="statusTag" size="large" class="status-tag">
          {{ backendInfo }}
        </el-tag>
        <el-button type="primary" @click="showCreateDialog = true">
          {{ t('firewall.add') }}
        </el-button>
        <el-button @click="handleApply">
          {{ t('firewall.apply') }}
        </el-button>
        <el-button :type="firewallEnabled ? 'danger' : 'success'" @click="handleToggleFirewall">
          {{ firewallEnabled ? t('firewall.disable') : t('firewall.enable') }}
        </el-button>
        <el-button :loading="loading" @click="loadData">
          {{ t('firewall.refresh') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :empty-text="t('common.noData')" :data="rules" stripe>
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="name" :label="t('firewall.name')" min-width="140" />
        <el-table-column :label="t('firewall.protocol')" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.protocol === 'any' ? 'info' : 'primary'">
              {{ row.protocol.toUpperCase() }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.port')" width="120">
          <template #default="{ row }">
            {{ row.port || t('firewall.any') }}
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.source')" width="140">
          <template #default="{ row }">
            {{ row.source || '0.0.0.0/0' }}
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.action')" width="90">
          <template #default="{ row }">
            <el-tag size="small" :type="row.action === 'allow' ? 'success' : 'danger'">
              {{ actionLabel(row.action) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.direction')" width="80">
          <template #default="{ row }">
            {{ row.direction === 'in' ? t('firewall.in') : t('firewall.out') }}
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.enabled')" width="80">
          <template #default="{ row }">
            <el-switch
              :model-value="row.enabled"
              @change="(v: boolean | string | number) => handleToggle(row.id, Boolean(v))"
            />
          </template>
        </el-table-column>
        <el-table-column :label="t('firewall.actions')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="handleEdit(row)">{{ t('common.edit') }}</el-button>
            <el-button size="small" type="danger" @click="handleDelete(row.id)">{{
              t('firewall.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="showCreateDialog" :title="t('firewall.add')" width="560px">
      <el-form :model="form" label-width="80px">
        <el-form-item :label="t('firewall.name')" required>
          <el-input v-model="form.name" :placeholder="t('firewall.namePlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('firewall.description')">
          <el-input v-model="form.description" :placeholder="t('firewall.descPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('firewall.protocol')" required>
          <el-select v-model="form.protocol" class="full-width">
            <el-option label="TCP" value="tcp" />
            <el-option label="UDP" value="udp" />
            <el-option label="ICMP" value="icmp" />
            <el-option :label="t('firewall.any')" value="any" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.port')">
          <el-input v-model="form.port" :placeholder="t('firewall.portPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('firewall.source')">
          <el-input v-model="form.source" placeholder="0.0.0.0/0" />
        </el-form-item>
        <el-form-item :label="t('firewall.action')" required>
          <el-select v-model="form.action" class="full-width">
            <el-option :label="t('firewall.allow') + ' (ALLOW)'" value="allow" />
            <el-option :label="t('firewall.deny') + ' (DENY)'" value="deny" />
            <el-option :label="t('firewall.reject') + ' (REJECT)'" value="reject" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.direction')" required>
          <el-select v-model="form.direction" class="full-width">
            <el-option :label="t('firewall.in')" value="in" />
            <el-option :label="t('firewall.out')" value="out" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.priority')">
          <el-input-number v-model="form.priority" :min="1" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateDialog = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="handleCreate">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showEditDialog" :title="t('firewall.edit')" width="560px">
      <el-form :model="editForm" label-width="80px">
        <el-form-item :label="t('firewall.name')" required>
          <el-input v-model="editForm.name" />
        </el-form-item>
        <el-form-item :label="t('firewall.description')">
          <el-input v-model="editForm.description" />
        </el-form-item>
        <el-form-item :label="t('firewall.protocol')" required>
          <el-select v-model="editForm.protocol" class="full-width">
            <el-option label="TCP" value="tcp" />
            <el-option label="UDP" value="udp" />
            <el-option label="ICMP" value="icmp" />
            <el-option :label="t('firewall.any')" value="any" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.port')">
          <el-input v-model="editForm.port" :placeholder="t('firewall.any')" />
        </el-form-item>
        <el-form-item :label="t('firewall.source')">
          <el-input v-model="editForm.source" />
        </el-form-item>
        <el-form-item :label="t('firewall.action')" required>
          <el-select v-model="editForm.action" class="full-width">
            <el-option :label="t('firewall.allow') + ' (ALLOW)'" value="allow" />
            <el-option :label="t('firewall.deny') + ' (DENY)'" value="deny" />
            <el-option :label="t('firewall.reject') + ' (REJECT)'" value="reject" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.direction')" required>
          <el-select v-model="editForm.direction" class="full-width">
            <el-option :label="t('firewall.in')" value="in" />
            <el-option :label="t('firewall.out')" value="out" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('firewall.priority')">
          <el-input-number v-model="editForm.priority" :min="1" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showEditDialog = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="handleSaveEdit">{{
          t('common.save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listFirewallRules,
  createFirewallRule,
  updateFirewallRule,
  deleteFirewallRule,
  toggleFirewallRule,
  applyFirewallRules,
  getFirewallStatus,
  enableFirewall,
  disableFirewall,
} from '@/api/firewall'
import type { FirewallRule as FR, FirewallStatus } from '@/types'

const { t } = useI18n()
const rules = ref<FR[]>([])
const loading = ref(false)
const saving = ref(false)
const showCreateDialog = ref(false)
const showEditDialog = ref(false)
const firewallStatus = ref<FirewallStatus | null>(null)

const form = ref({
  name: '',
  description: '',
  protocol: 'tcp',
  port: '',
  source: '0.0.0.0/0',
  action: 'allow',
  direction: 'in',
  priority: 50,
})

const editForm = ref<FR>({} as FR)

function actionLabel(action: string) {
  const map: Record<string, string> = {
    allow: t('firewall.allow'),
    deny: t('firewall.deny'),
    reject: t('firewall.reject'),
  }
  return map[action] || action
}

const statusTag = computed(() => {
  if (!firewallStatus.value) return 'info'
  const s = (firewallStatus.value.status || '').toLowerCase()
  if (s.includes('active') || s.includes('running') || s.includes('enable')) return 'success'
  if (s.includes('inactive') || s.includes('disable')) return 'danger'
  return 'warning'
})

const backendInfo = computed(() => {
  if (!firewallStatus.value) return t('firewall.detecting')
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
    rules.value = ruleRes.data.data
    if (statusRes) firewallStatus.value = statusRes.data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  if (!form.value.name) {
    ElMessage.warning(t('common.required'))
    return
  }
  saving.value = true
  try {
    const data: Record<string, string | number> = {
      name: form.value.name,
      protocol: form.value.protocol,
      action: form.value.action,
      direction: form.value.direction,
      priority: form.value.priority,
      source: form.value.source || '0.0.0.0/0',
    }
    if (form.value.description) data.description = form.value.description
    if (form.value.port) data.port = form.value.port
    await createFirewallRule(data)
    ElMessage.success(t('common.success'))
    showCreateDialog.value = false
    form.value = {
      name: '',
      description: '',
      protocol: 'tcp',
      port: '',
      source: '0.0.0.0/0',
      action: 'allow',
      direction: 'in',
      priority: 50,
    }
    await loadData()
  } catch {
    ElMessage.error(t('common.failed'))
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
    const data: Record<string, string | number> = {}
    if (editForm.value.name) data.name = editForm.value.name
    if (editForm.value.description) data.description = editForm.value.description
    data.protocol = editForm.value.protocol
    if (editForm.value.port) data.port = editForm.value.port
    data.source = editForm.value.source || '0.0.0.0/0'
    data.action = editForm.value.action
    data.direction = editForm.value.direction
    data.priority = editForm.value.priority
    await updateFirewallRule(editForm.value.id, data)
    ElMessage.success(t('common.success'))
    showEditDialog.value = false
    await loadData()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    saving.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm(t('common.confirmAction'), t('common.confirm'))
  } catch {
    return
  }
  try {
    await deleteFirewallRule(id)
    ElMessage.success(t('common.success'))
    await loadData()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleToggle(id: number, enabled: boolean) {
  try {
    await toggleFirewallRule(id, enabled)
    ElMessage.success(t('common.success'))
    await loadData()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleApply() {
  try {
    await applyFirewallRules()
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function handleToggleFirewall() {
  try {
    if (firewallEnabled.value) {
      await disableFirewall()
      ElMessage.success(t('common.success'))
    } else {
      await enableFirewall()
      ElMessage.success(t('common.success'))
    }
    await loadData()
  } catch {
    ElMessage.error(t('common.failed'))
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
