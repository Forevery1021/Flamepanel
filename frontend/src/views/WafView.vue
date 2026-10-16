<template>
  <LayoutContent
    :title="t('waf.title')"
    :description="t('pageDesc.waf')"
    reload
    @reload="refreshAll"
  >
    <template #toolbar>
      <FpSelect
        v-model="websiteFilter"
        :options="websiteOptions"
        option-label="label"
        option-value="value"
        class="toolbar-filter"
      />
    </template>
    <template #actions>
      <FpTag :severity="'info'" :value="engineTag" class="status-tag" />
      <FpButton
        v-permission="{ perm: 'waf:update', mode: 'view' }"
        variant="ghost"
        icon="oi oi-share-boxed"
        :loading="applying"
        @click="onApply"
      >
        {{ t('waf.apply') }}
      </FpButton>
      <FpButton
        v-permission="{ perm: 'waf:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="openDialog()"
      >
        {{ t('waf.addRule') }}
      </FpButton>
    </template>

    <FpTabs v-model="activeTab" :items="tabItems">
      <template #rules>
        <div v-if="applyReport" class="apply-report" role="status">
          <div class="apply-report__head">
            <strong>{{ t('waf.applyReport') }}</strong>
            <FpButton variant="link" @click="applyReport = null">{{ t('common.close') }}</FpButton>
          </div>
          <div v-for="e in applyReport.engines" :key="e.engine" class="apply-report__row">
            <code class="apply-report__engine">{{ e.engine }}</code>
            <FpTag :severity="engineSeverity(e)" :value="engineState(e)" />
            <span v-if="e.applied">{{ t('waf.applyEnforced', { n: e.enforced.length }) }}</span>
            <span v-if="e.skipped.length" class="apply-report__muted">
              {{ t('waf.applySkipped', { n: e.skipped.length }) }}:
              {{ e.skipped.map((s) => `#${s.rule_id} ${s.reason}`).join('；') }}
            </span>
            <span v-if="e.redeployed_sites.length" class="apply-report__muted">
              {{ t('waf.applyRedeployed', { sites: e.redeployed_sites.join(', ') }) }}
            </span>
            <span v-if="e.error || e.reason" class="apply-report__muted">{{
              e.error || e.reason
            }}</span>
          </div>
        </div>
        <div class="panel">
          <FpTable :rows="rules" :loading="rulesLoading" :empty-text="t('common.noData')">
            <FpColumn field="name" :header="t('waf.name')" style="min-width: 140px" />
            <FpColumn :header="t('waf.type')" style="width: 130px">
              <template #body="{ data }">
                <FpTag severity="info" :value="typeLabel(data.rule_type)" />
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.website')" style="width: 140px">
              <template #body="{ data }">
                <span>{{
                  data.website_id > 0 ? websiteName(data.website_id) : t('waf.global')
                }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.pattern')" style="min-width: 140px">
              <template #body="{ data }">
                <code class="pattern-code">{{ data.pattern || t('waf.builtin') }}</code>
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.action')" style="width: 90px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.action === 'block' ? 'danger' : 'warning'"
                  :value="data.action === 'block' ? t('waf.block') : t('waf.logOnly')"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.enabled')" style="width: 90px">
              <template #body="{ data }">
                <FpSwitch
                  :model-value="data.enabled"
                  @update:model-value="(v: boolean) => onToggle(data, v)"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('common.colActions')" style="width: 160px" frozen>
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton
                    v-permission="{ perm: 'waf:update', mode: 'view' }"
                    variant="link"
                    @click="openDialog(data)"
                  >
                    {{ t('common.edit') }}
                  </FpButton>
                  <FpButton
                    v-permission="{ perm: 'waf:delete', mode: 'view' }"
                    variant="link"
                    @click="onDelete(data)"
                  >
                    {{ t('common.delete') }}
                  </FpButton>
                </div>
              </template>
            </FpColumn>
          </FpTable>
        </div>
      </template>

      <template #logs>
        <div class="panel">
          <div class="logs-toolbar">
            <div class="ingest-status" role="status">
              <template v-if="ingest.length">
                <span v-for="i in ingest" :key="i.engine" class="ingest-status__item">
                  <FpTag :severity="i.last_error ? 'danger' : 'success'" :value="i.engine" />
                  <span v-if="i.last_error" class="ingest-status__muted">{{
                    t('waf.ingestError', { error: i.last_error })
                  }}</span>
                  <span v-else class="ingest-status__muted">{{
                    t('waf.ingestOk', {
                      n: i.imported,
                      time: i.last_run_at ? formatTime(i.last_run_at) : '-',
                    })
                  }}</span>
                </span>
              </template>
              <span v-else class="ingest-status__muted">{{ t('waf.ingestNone') }}</span>
            </div>
            <FpButton
              v-permission="{ perm: 'waf:update', mode: 'view' }"
              variant="ghost"
              icon="oi oi-trash"
              @click="onClearLogs"
            >
              {{ t('waf.clearLogs') }}
            </FpButton>
          </div>
          <FpTable :rows="logs" :loading="logsLoading" :empty-text="t('common.noData')">
            <FpColumn :header="t('waf.time')" style="width: 170px">
              <template #body="{ data }">
                <span>{{ new Date(data.created_at).toLocaleString() }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.clientIp')" style="width: 140px">
              <template #body="{ data }"
                ><code>{{ data.client_ip }}</code></template
              >
            </FpColumn>
            <FpColumn field="rule_name" :header="t('waf.ruleName')" style="width: 150px" />
            <FpColumn :header="t('waf.type')" style="width: 120px">
              <template #body="{ data }">
                <FpTag severity="info" :value="typeLabel(data.rule_type)" />
              </template>
            </FpColumn>
            <FpColumn :header="t('waf.action')" style="width: 90px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.action === 'block' ? 'danger' : 'warning'"
                  :value="data.action === 'block' ? t('waf.block') : t('waf.logOnly')"
                />
              </template>
            </FpColumn>
            <FpColumn field="request_summary" :header="t('waf.request')" style="min-width: 180px">
              <template #body="{ data }">
                <code class="pattern-code">{{ data.request_summary }}</code>
              </template>
            </FpColumn>
            <FpColumn field="user_agent" :header="'UA'" style="min-width: 120px" />
          </FpTable>
          <div v-if="logsTotal > logsPageSize" class="pagination-bar">
            <FpButton variant="ghost" :disabled="logsPage <= 1" @click="logsPage--">
              {{ t('common.prev') }}
            </FpButton>
            <span class="page-indicator"
              >{{ logsPage }} / {{ Math.ceil(logsTotal / logsPageSize) }}</span
            >
            <FpButton
              variant="ghost"
              :disabled="logsPage >= Math.ceil(logsTotal / logsPageSize)"
              @click="logsPage++"
            >
              {{ t('common.next') }}
            </FpButton>
          </div>
        </div>
      </template>
    </FpTabs>

    <!-- 创建/编辑规则弹窗 -->
    <FpModal
      v-model="dialogVisible"
      :header="editingId ? t('waf.editRule') : t('waf.addRule')"
      size="md"
    >
      <div class="modal-form">
        <FpInput v-model="form.name" :label="t('waf.name')" :error="errors.name" />
        <FpSelect
          v-model="form.rule_type"
          :label="t('waf.type')"
          :options="ruleTypeOptions"
          option-label="label"
          option-value="value"
        />
        <FpSelect
          v-model="form.website_id"
          :label="t('waf.website')"
          :options="websiteSelectOptions"
          option-label="label"
          option-value="value"
        />
        <FpInput
          v-if="needsPattern"
          v-model="form.pattern"
          :label="t('waf.pattern')"
          :placeholder="patternPlaceholder"
          :error="errors.pattern"
        />
        <template v-if="form.rule_type === 'cc'">
          <div class="field-inline">
            <FpNumber v-model="ccThreshold" :min="1" :max="100000" style="width: 160px" />
            <span class="hint">{{ t('waf.ccThreshold') }}</span>
          </div>
          <div class="field-inline">
            <FpNumber v-model="ccWindow" :min="1" :max="3600" style="width: 160px" />
            <span class="hint">{{ t('waf.ccWindow') }}</span>
          </div>
        </template>
        <FpSelect
          v-model="form.action"
          :label="t('waf.action')"
          :options="actionOptions"
          option-label="label"
          option-value="value"
        />
        <small class="field-hint">{{ t('waf.actionHint') }}</small>
        <FpSwitch v-model="form.enabled" :label="t('waf.enabled')" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="saving" @click="save">{{
          t('common.save')
        }}</FpButton>
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  listWafRules,
  createWafRule,
  updateWafRule,
  deleteWafRule,
  listWafLogs,
  clearWafLogs,
  getWafStatus,
  applyWafRules,
} from '@/api/waf'
import { listWebsites } from '@/api/websites'
import type {
  WafRule,
  WafLog,
  Page,
  Website,
  WafApplyReport,
  WafEngineApply,
  WafEngineStatus,
  WafIngestStatus,
} from '@/api/generated'

interface WafLogPage {
  data: WafLog[]
  page: number
  page_size: number
  total: number
  total_pages: number
}
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const queryClient = useQueryCacheClient()

const activeTab = ref<'rules' | 'logs'>('rules')
const tabItems = computed(() => [
  { value: 'rules', label: t('waf.rulesTab') },
  { value: 'logs', label: t('waf.logsTab') },
])

/* ── 网站下拉 ── */
const websitesQuery = useApiQuery<Page<Website>>(
  () => queryKeys.websites.list(1, 200),
  async () => {
    const res = await listWebsites(1, 200)
    return { data: res.data }
  },
)

const websites = computed<Website[]>(() => websitesQuery.data.value?.data ?? [])

function websiteName(id: number): string {
  const w = websites.value.find((x) => x.id === id)
  return w ? w.domain : `#${id}`
}

const websiteOptions = computed(() => [
  { label: t('waf.allWebsites'), value: 0 },
  ...websites.value.map((w) => ({ label: w.domain, value: w.id })),
])

const websiteSelectOptions = computed(() => [
  { label: t('waf.globalRule'), value: 0 },
  ...websites.value.map((w) => ({ label: w.domain, value: w.id })),
])

/* ── 引擎状态 ── */
const statusQuery = useApiQuery<WafEngineStatus>(
  () => queryKeys.waf.status(),
  async () => {
    const res = await getWafStatus()
    return { data: res.data }
  },
)
const ingest = computed<WafIngestStatus[]>(() => statusQuery.data.value?.ingest ?? [])

function formatTime(iso: string): string {
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleTimeString()
}

const engineTag = computed(() =>
  t('waf.engineRules').replace('{n}', String(statusQuery.data.value?.loaded_rules ?? 0)),
)

/* ── 规则列表 ── */
const websiteFilter = ref(0)
const rulesQuery = useApiQuery<WafRule[]>(
  () => queryKeys.waf.rules(),
  async () => {
    const res = await listWafRules(0)
    return { data: res.data }
  },
)
const rules = computed<WafRule[]>(() => {
  const all = rulesQuery.data.value ?? []
  return websiteFilter.value > 0
    ? all.filter((r) => r.website_id === 0 || r.website_id === websiteFilter.value)
    : all
})
const rulesLoading = rulesQuery.loading

/* ── 拦截日志 ── */
const logsPage = ref(1)
const logsPageSize = 20
const logsQuery = useApiQuery<WafLogPage>(
  () => queryKeys.waf.logs(logsPage.value, websiteFilter.value),
  async () => {
    const res = await listWafLogs(logsPage.value, logsPageSize, websiteFilter.value)
    return { data: res.data }
  },
)
const logs = computed<WafLog[]>(() => logsQuery.data.value?.data ?? [])
const logsTotal = computed(() => logsQuery.data.value?.total ?? 0)
const logsLoading = logsQuery.loading

watch(websiteFilter, () => {
  logsPage.value = 1
})

function refreshAll() {
  rulesQuery.refresh()
  logsQuery.refresh()
  statusQuery.refresh()
}

const ruleTypeOptions = computed(() => [
  { label: t('waf.typeCc'), value: 'cc' },
  { label: t('waf.typeSqli'), value: 'sql_injection' },
  { label: t('waf.typeXss'), value: 'xss' },
  { label: t('waf.typeIpBlack'), value: 'ip_blacklist' },
  { label: t('waf.typeIpWhite'), value: 'ip_whitelist' },
  { label: t('waf.typeUaBlack'), value: 'ua_blacklist' },
  { label: t('waf.typeUaWhite'), value: 'ua_whitelist' },
  { label: t('waf.typePath'), value: 'path_block' },
])

function typeLabel(type: string): string {
  return ruleTypeOptions.value.find((r) => r.value === type)?.label ?? type
}

const actionOptions = computed(() => [
  { label: t('waf.logOnly'), value: 'log' },
  { label: t('waf.block'), value: 'block' },
])

/* ── 创建/编辑弹窗 ── */
const dialogVisible = ref(false)
const saving = ref(false)
const editingId = ref(0)
const form = reactive({
  name: '',
  rule_type: 'ip_blacklist',
  website_id: 0,
  pattern: '',
  action: 'log',
  enabled: true,
})
const ccThreshold = ref(120)
const ccWindow = ref(60)
const errors = reactive({ name: '', pattern: '' })

const needsPattern = computed(() => !['sql_injection', 'xss', 'cc'].includes(form.rule_type))

const patternPlaceholder = computed(() => {
  const map: Record<string, string> = {
    ip_blacklist: '1.2.3.4, 10.0.0.0/8',
    ip_whitelist: '192.168.1.0/24',
    ua_blacklist: 'sqlmap',
    ua_whitelist: 'Googlebot',
    path_block: '/wp-admin',
  }
  return map[form.rule_type] ?? ''
})

function openDialog(row?: WafRule) {
  editingId.value = row?.id ?? 0
  form.name = row?.name ?? ''
  form.rule_type = row?.rule_type ?? 'ip_blacklist'
  form.website_id = row?.website_id ?? 0
  form.pattern = row?.pattern ?? ''
  form.action = row?.action ?? 'log'
  form.enabled = row?.enabled ?? true
  const cfg = row?.config_json ? safeParse(row.config_json) : {}
  ccThreshold.value = typeof cfg.threshold === 'number' ? cfg.threshold : 120
  ccWindow.value = typeof cfg.window_secs === 'number' ? cfg.window_secs : 60
  errors.name = ''
  errors.pattern = ''
  dialogVisible.value = true
}

function safeParse(s: string): Record<string, unknown> {
  try {
    return JSON.parse(s) as Record<string, unknown>
  } catch {
    return {}
  }
}

function validate(): boolean {
  errors.name = form.name.trim() ? '' : t('common.required')
  errors.pattern = needsPattern.value && !form.pattern.trim() ? t('common.required') : ''
  return !errors.name && !errors.pattern
}

async function save() {
  if (!validate()) return
  saving.value = true
  try {
    const payload = {
      name: form.name.trim(),
      rule_type: form.rule_type,
      website_id: form.website_id,
      pattern: form.pattern.trim(),
      config_json: JSON.stringify(
        form.rule_type === 'cc'
          ? { threshold: ccThreshold.value, window_secs: ccWindow.value }
          : {},
      ),
      action: form.action,
      enabled: form.enabled,
    }
    if (editingId.value) {
      await updateWafRule(editingId.value, payload)
    } else {
      await createWafRule(payload)
    }
    toast.success(t('common.success'))
    dialogVisible.value = false
    queryClient.invalidateQueries({ queryKey: queryKeys.waf.all })
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    saving.value = false
  }
}

function onToggle(row: WafRule, enabled: boolean) {
  confirmAction({
    message: enabled
      ? t('waf.enableConfirm', { name: row.name })
      : t('waf.disableConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await updateWafRule(row.id, { enabled })
        toast.success(t('common.success'))
        queryClient.invalidateQueries({ queryKey: queryKeys.waf.all })
      } catch {
        toast.error(t('common.failed'))
        queryClient.invalidateQueries({ queryKey: queryKeys.waf.all })
      }
    },
  })
}

function onDelete(row: WafRule) {
  confirmAction({
    message: t('waf.deleteConfirm', { name: row.name }),
    header: t('common.confirm'),
    acceptLabel: t('common.delete'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await deleteWafRule(row.id)
        toast.success(t('common.success'))
        queryClient.invalidateQueries({ queryKey: queryKeys.waf.all })
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

// ── Web 引擎下发（Doc/32 W3）──
const applying = ref(false)
const applyReport = ref<WafApplyReport | null>(null)

function engineState(e: WafEngineApply): string {
  if (e.error) return t('waf.applyFailed')
  if (!e.applied) return t('waf.applyNotAvailable')
  return e.reloaded ? t('waf.applyReloaded') : t('waf.applyPending')
}

function engineSeverity(e: WafEngineApply): 'success' | 'warning' | 'danger' | 'info' {
  if (e.error) return 'danger'
  if (!e.applied) return 'info'
  return e.reloaded ? 'success' : 'warning'
}

async function onApply() {
  applying.value = true
  try {
    const { data } = await applyWafRules()
    applyReport.value = data
    if (data.ok) toast.success(t('waf.applyDone'))
    else toast.warning(t('waf.applyPartial'))
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    applying.value = false
  }
}

function onClearLogs() {
  confirmAction({
    message: t('waf.clearConfirm'),
    header: t('common.confirm'),
    acceptLabel: t('waf.clearLogs'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await clearWafLogs(websiteFilter.value)
        toast.success(t('common.success'))
        queryClient.invalidateQueries({ queryKey: queryKeys.waf.all })
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}
</script>

<style scoped>
.apply-report {
  border: 1px solid var(--fp-border-color, rgba(128, 128, 128, 0.3));
  border-radius: 8px;
  padding: 0.75rem 1rem;
  margin-bottom: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.apply-report__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.apply-report__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}
.apply-report__engine {
  min-width: 80px;
}
.apply-report__muted {
  opacity: 0.75;
  word-break: break-all;
}
.ingest-status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  margin-right: auto;
  font-size: 0.85rem;
}
.ingest-status__item {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}
.ingest-status__muted {
  opacity: 0.75;
}
.logs-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  margin-bottom: 0.75rem;
}
.pattern-code {
  font-size: 0.85rem;
  word-break: break-all;
}
.pagination-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 0.75rem;
}
.page-indicator {
  font-size: 0.85rem;
}
</style>
