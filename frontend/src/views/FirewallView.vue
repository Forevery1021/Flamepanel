<template>
  <LayoutContent :title="t('firewall.title')" reload @reload="loadData">
    <template #toolbar>
      <FpTag :severity="statusTag" :value="backendInfo" class="status-tag" />
      <FpButton variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('firewall.add') }}
      </FpButton>
      <FpButton variant="ghost" icon="oi oi-sync" @click="handleApply">
        {{ t('firewall.apply') }}
      </FpButton>
      <FpButton
        :variant="firewallEnabled ? 'danger' : 'success'"
        icon="oi oi-power-off"
        @click="handleToggleFirewall"
      >
        {{ firewallEnabled ? t('firewall.disable') : t('firewall.enable') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable
        :rows="rules"
        :loading="loading"
        :first="(currentPage - 1) * pageSize"
        :rows-per-page="pageSize"
        :empty-text="t('common.noData')"
        @update:first="goPage"
      >
        <Column header="#" style="width: 50px">
          <template #body="{ index }">{{ index + 1 }}</template>
        </Column>
        <Column field="name" :header="t('firewall.name')" />
        <Column :header="t('firewall.protocol')" style="width: 90px">
          <template #body="{ data }">
            <FpTag
              :severity="data.protocol === 'any' ? 'neutral' : 'info'"
              :value="data.protocol.toUpperCase()"
            />
          </template>
        </Column>
        <Column :header="t('firewall.port')" style="width: 120px">
          <template #body="{ data }">{{ data.port || t('firewall.any') }}</template>
        </Column>
        <Column :header="t('firewall.source')" style="width: 150px">
          <template #body="{ data }">{{ data.source || '0.0.0.0/0' }}</template>
        </Column>
        <Column :header="t('firewall.action')" style="width: 100px">
          <template #body="{ data }">
            <FpTag
              :severity="data.action === 'allow' ? 'success' : 'danger'"
              :value="actionLabel(data.action)"
            />
          </template>
        </Column>
        <Column :header="t('firewall.direction')" style="width: 90px">
          <template #body="{ data }">
            {{ data.direction === 'in' ? t('firewall.in') : t('firewall.out') }}
          </template>
        </Column>
        <Column :header="t('firewall.enabled')" style="width: 90px">
          <template #body="{ data }">
            <ToggleSwitch
              :model-value="data.enabled"
              @update:model-value="(v: boolean) => handleToggle(data.id, v)"
            />
          </template>
        </Column>
        <Column :header="t('firewall.actions')" style="width: 150px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton variant="link" @click="handleDelete(data.id)">
                {{ t('firewall.delete') }}
              </FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
    </div>

    <FpModal v-model="showCreateDialog" :header="t('firewall.add')" style="width: 560px">
      <div class="modal-form">
        <FpInput
          v-model="form.name"
          :label="t('firewall.name')"
          :placeholder="t('firewall.namePlaceholder')"
          :error="formErrors.name"
        />
        <FpInput
          v-model="form.description"
          :label="t('firewall.description')"
          :placeholder="t('firewall.descPlaceholder')"
        />
        <FpSelect
          v-model="form.protocol"
          :label="t('firewall.protocol')"
          :options="protocolOptions"
          option-label="label"
          option-value="value"
        />
        <FpInput
          v-model="form.port"
          :label="t('firewall.port')"
          :placeholder="t('firewall.portPlaceholder')"
        />
        <FpInput v-model="form.source" :label="t('firewall.source')" placeholder="0.0.0.0/0" />
        <FpSelect
          v-model="form.action"
          :label="t('firewall.action')"
          :options="actionOptions"
          option-label="label"
          option-value="value"
        />
        <FpSelect
          v-model="form.direction"
          :label="t('firewall.direction')"
          :options="directionOptions"
          option-label="label"
          option-value="value"
        />
        <div class="field-col">
          <label class="field-label">{{ t('firewall.priority') }}</label>
          <InputNumber v-model="form.priority" :min="1" :max="999" class="w-full" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showCreateDialog = false">
          {{ t('common.cancel') }}
        </FpButton>
        <FpButton variant="primary" :loading="saving" @click="handleCreate">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showEditDialog" :header="t('firewall.edit')" style="width: 560px">
      <div class="modal-form">
        <FpInput v-model="editForm.name" :label="t('firewall.name')" :error="editErrors.name" />
        <FpInput v-model="editForm.description" :label="t('firewall.description')" />
        <FpSelect
          v-model="editForm.protocol"
          :label="t('firewall.protocol')"
          :options="protocolOptions"
          option-label="label"
          option-value="value"
        />
        <FpInput v-model="editForm.port" :label="t('firewall.port')" :placeholder="t('firewall.any')" />
        <FpInput v-model="editForm.source" :label="t('firewall.source')" />
        <FpSelect
          v-model="editForm.action"
          :label="t('firewall.action')"
          :options="actionOptions"
          option-label="label"
          option-value="value"
        />
        <FpSelect
          v-model="editForm.direction"
          :label="t('firewall.direction')"
          :options="directionOptions"
          option-label="label"
          option-value="value"
        />
        <div class="field-col">
          <label class="field-label">{{ t('firewall.priority') }}</label>
          <InputNumber v-model="editForm.priority" :min="1" :max="999" class="w-full" />
        </div>
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showEditDialog = false">
          {{ t('common.cancel') }}
        </FpButton>
        <FpButton variant="primary" :loading="saving" @click="handleSaveEdit">
          {{ t('common.save') }}
        </FpButton>
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import InputNumber from 'openvue/inputnumber'
import ToggleSwitch from 'openvue/toggleswitch'
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
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

type TagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const rules = ref<FR[]>([])
const loading = ref(false)
const saving = ref(false)
const currentPage = ref(1)
const pageSize = ref(10)
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

type EditForm = FR & { description: string; port: string; source: string }

const editForm = ref<EditForm>({} as EditForm)
const formErrors = reactive({ name: '' })
const editErrors = reactive({ name: '' })

const protocolOptions = computed(() => [
  { label: 'TCP', value: 'tcp' },
  { label: 'UDP', value: 'udp' },
  { label: 'ICMP', value: 'icmp' },
  { label: t('firewall.any'), value: 'any' },
])

const actionOptions = computed(() => [
  { label: `${t('firewall.allow')} (ALLOW)`, value: 'allow' },
  { label: `${t('firewall.deny')} (DENY)`, value: 'deny' },
  { label: `${t('firewall.reject')} (REJECT)`, value: 'reject' },
])

const directionOptions = computed(() => [
  { label: t('firewall.in'), value: 'in' },
  { label: t('firewall.out'), value: 'out' },
])

function actionLabel(action: string) {
  const map: Record<string, string> = {
    allow: t('firewall.allow'),
    deny: t('firewall.deny'),
    reject: t('firewall.reject'),
  }
  return map[action] || action
}

const statusTag = computed<TagSeverity>(() => {
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
    toast.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
}

function openCreate() {
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
  formErrors.name = ''
  showCreateDialog.value = true
}

function validateForm(): boolean {
  formErrors.name = form.value.name ? '' : t('common.required')
  return !formErrors.name
}

async function handleCreate() {
  if (!validateForm()) return
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
    toast.success(t('common.success'))
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
    toast.error(t('common.failed'))
  } finally {
    saving.value = false
  }
}

function handleEdit(rule: FR) {
  editForm.value = {
    ...rule,
    description: rule.description ?? '',
    port: rule.port ?? '',
    source: rule.source ?? '',
  }
  editErrors.name = ''
  showEditDialog.value = true
}

function validateEditForm(): boolean {
  editErrors.name = editForm.value.name ? '' : t('common.required')
  return !editErrors.name
}

async function handleSaveEdit() {
  if (!validateEditForm()) return
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
    toast.success(t('common.success'))
    showEditDialog.value = false
    await loadData()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    saving.value = false
  }
}

function handleDelete(id: number) {
  confirmAction({
    message: t('common.confirmAction'),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await deleteFirewallRule(id)
        toast.success(t('common.success'))
        await loadData()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
}

async function handleToggle(id: number, enabled: boolean) {
  try {
    await toggleFirewallRule(id, enabled)
    toast.success(t('common.success'))
    await loadData()
  } catch {
    toast.error(t('common.failed'))
  }
}

async function handleApply() {
  try {
    await applyFirewallRules()
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

async function handleToggleFirewall() {
  try {
    if (firewallEnabled.value) {
      await disableFirewall()
      toast.success(t('common.success'))
    } else {
      await enableFirewall()
      toast.success(t('common.success'))
    }
    await loadData()
  } catch {
    toast.error(t('common.failed'))
  }
}

onMounted(loadData)
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.status-tag {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
</style>
