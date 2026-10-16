<template>
  <LayoutContent
    :title="t('firewall.title')"
    :description="t('pageDesc.firewall')"
    reload
    @reload="loadData"
  >
    <template #toolbar>
      <div class="toolbar-left">
        <FpInput
          v-model="searchText"
          :placeholder="t('common.searchPlaceholder')"
          class="toolbar-search"
        />
        <FpSelect
          v-model="protoFilter"
          :options="protoFilterOptions"
          option-label="label"
          option-value="value"
          show-clear
          class="toolbar-filter"
        />
      </div>
      <div class="toolbar-right">
        <FpTag :severity="statusTag" :value="backendInfo" class="status-tag" />
        <FpButton
          v-permission="{ perm: 'firewall:create', mode: 'view' }"
          variant="primary"
          icon="oi oi-plus"
          class="add-rule"
          @click="openCreate"
        >
          {{ t('firewall.add') }}
        </FpButton>
        <FpButton
          v-permission="{ perm: 'firewall:apply', mode: 'view' }"
          variant="ghost"
          icon="oi oi-sync"
          @click="handleApply"
        >
          {{ t('firewall.apply') }}
        </FpButton>
        <FpButton
          v-permission="{ perm: 'firewall:enable', mode: 'view' }"
          :variant="firewallEnabled ? 'danger' : 'success'"
          icon="oi oi-power-off"
          @click="handleToggleFirewall"
        >
          {{ firewallEnabled ? t('firewall.disable') : t('firewall.enable') }}
        </FpButton>
      </div>
    </template>

    <div class="panel">
      <FirewallRuleTable
        :rules="filteredRules"
        :loading="loading"
        :first="first"
        :page-size="pageSize"
        @update:first="onFirst"
        @create="openCreate"
        @edit="openEdit"
        @delete="handleDelete"
        @toggle="handleToggle"
      />
    </div>

    <FirewallRuleModal
      v-model:open="modalOpen"
      :mode="editing ? 'edit' : 'create'"
      :rule="editing"
      :backend="firewallStatus?.backend_name ?? null"
      :saving="saving"
      @submit="handleSubmit"
    />
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  applyFirewallRules,
  createFirewallRule,
  deleteFirewallRule,
  disableFirewall,
  enableFirewall,
  getFirewallStatus,
  listFirewallRules,
  toggleFirewallRule,
  updateFirewallRule,
} from '@/api/firewall'
import type { FirewallRule, FirewallStatus } from '@/types'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FirewallRuleTable from '@/components/firewall/FirewallRuleTable.vue'
import FirewallRuleModal from '@/components/firewall/FirewallRuleModal.vue'
import { useCrudPage } from '@/composables/useCrudPage'
import { firewallStatusView } from '@/utils/firewallRule'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const rules = ref<FirewallRule[]>([])
const loading = ref(false)
const saving = ref(false)
const firewallStatus = ref<FirewallStatus | null>(null)
// Y9：分页状态收敛到 useCrudPage
const { first, pageSize, onFirst } = useCrudPage({ pageSize: 10 })

// M9：搜索 + 协议筛选（客户端过滤）
const searchText = ref('')
const protoFilter = ref<string>('')
const protoFilterOptions = [
  { label: 'TCP', value: 'tcp' },
  { label: 'UDP', value: 'udp' },
  { label: 'ICMP', value: 'icmp' },
]
const filteredRules = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return rules.value.filter((r) => {
    if (protoFilter.value && r.protocol !== protoFilter.value) return false
    if (!kw) return true
    return [r.name, r.description, r.port, r.source].some((v) =>
      (v ?? '').toLowerCase().includes(kw),
    )
  })
})

const status = computed(() => firewallStatusView(firewallStatus.value))
const firewallEnabled = computed(() => status.value.enabled)
const statusTag = computed(() => status.value.severity)
const backendInfo = computed(() => status.value.label ?? t('firewall.detecting'))

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

// ── 新建 / 编辑（共用弹窗） ────────────────────────────────
const modalOpen = ref(false)
const editing = ref<FirewallRule | null>(null)

function openCreate() {
  editing.value = null
  modalOpen.value = true
}

function openEdit(rule: FirewallRule) {
  editing.value = rule
  modalOpen.value = true
}

async function handleSubmit(payload: Record<string, string | number>) {
  saving.value = true
  try {
    if (editing.value) await updateFirewallRule(editing.value.id, payload)
    else await createFirewallRule(payload)
    toast.success(t('common.success'))
    modalOpen.value = false
    await loadData()
  } catch (e) {
    // 前端已与后端同口径校验；仍被拒（如 400 / 系统命令失败）时展示后端原因
    toast.error(e, t('common.failed'))
  } finally {
    saving.value = false
  }
}

function handleDelete(rule: FirewallRule) {
  confirmAction({
    message: t('firewall.deleteConfirm', { name: rule.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await deleteFirewallRule(rule.id)
        toast.success(t('common.success'))
        await loadData()
      } catch (e) {
        toast.error(e, t('common.failed'))
      }
    },
  })
}

async function runAndReload(action: () => Promise<unknown>, reload = true) {
  try {
    await action()
    toast.success(t('common.success'))
    if (reload) await loadData()
  } catch (e) {
    toast.error(e, t('common.failed'))
  }
}

const handleToggle = (rule: FirewallRule, enabled: boolean) =>
  runAndReload(() => toggleFirewallRule(rule.id, enabled))
const handleApply = () => runAndReload(applyFirewallRules, false)
const handleToggleFirewall = () =>
  runAndReload(() => (firewallEnabled.value ? disableFirewall() : enableFirewall()))

onMounted(loadData)
</script>

<style scoped>
.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.toolbar-search {
  width: 240px;
}
.toolbar-filter {
  width: 160px;
}
.status-tag {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
