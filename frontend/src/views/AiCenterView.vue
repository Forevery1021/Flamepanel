<template>
  <LayoutContent :title="t('aiCenter.title')" :description="t('pageDesc.aiCenter')" reload @reload="refreshAll">
    <template #toolbar>
      <FpButton
        v-if="canCreate"
        icon="oi oi-plus"
        :label="t('aiCenter.issue')"
        @click="openCreate"
      />
    </template>

    <FpTabs v-model="tab" :items="tabItems">
      <!-- 凭证 -->
      <template #credentials>
        <div class="panel">
          <FpTable
            :rows="principals"
            :loading="principalsLoading"
            :empty-text="t('aiCenter.noPrincipals')"
            striped-rows
          >
            <FpColumn field="name" :header="t('aiCenter.name')" style="min-width: 160px" />
            <FpColumn :header="t('aiCenter.tokenPrefix')" style="width: 150px">
              <template #body="{ data }">
                <span class="mono">{{ data.token_prefix }}…</span>
              </template>
            </FpColumn>
            <FpColumn field="owner" :header="t('aiCenter.owner')" style="width: 120px" />
            <FpColumn :header="t('aiCenter.access')" style="width: 130px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.allow_write ? 'warning' : 'neutral'"
                  :value="data.allow_write ? t('aiCenter.readWrite') : t('aiCenter.readOnly')"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.state')" style="width: 110px">
              <template #body="{ data }">
                <FpTag :severity="stateSeverity(data)" :value="t(`aiCenter.states.${principalState(data)}`)" />
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.expiresAt')" style="width: 180px">
              <template #body="{ data }">
                <span class="mono">{{ data.expires_at || t('aiCenter.never') }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.lastUsedAt')" style="width: 180px">
              <template #body="{ data }">
                <span class="mono">{{ data.last_used_at || '—' }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('common.operation')" style="width: 150px">
              <template #body="{ data }">
                <div class="row-actions">
                  <FpButton
                    variant="ghost"
                    size="small"
                    icon="oi oi-list"
                    :title="t('aiCenter.viewCalls')"
                    :aria-label="t('aiCenter.viewCalls')"
                    @click="showCallsOf(data.id)"
                  />
                  <FpButton
                    v-if="canRevoke(data)"
                    variant="danger"
                    size="small"
                    text
                    :label="t('aiCenter.revoke')"
                    @click="confirmRevoke(data)"
                  />
                </div>
              </template>
            </FpColumn>
          </FpTable>
        </div>

        <div class="panel snippets">
          <h3 class="section-title">{{ t('aiCenter.connectTitle') }}</h3>
          <p class="muted">{{ t('aiCenter.connectHint') }}</p>
          <McpSnippetBlock :url="endpointUrl" />
        </div>
      </template>

      <!-- 写操作审批（M-AI-2） -->
      <template #approvals>
        <ApprovalsPanel :principal-name="principalName" :tools="tools" @changed="onApprovalsChanged" />
      </template>

      <!-- 调用记录 -->
      <template #calls>
        <div class="panel">
          <div class="filters">
            <FpSelect
              v-model="principalFilter"
              :options="principalOptions"
              option-label="label"
              option-value="value"
              :aria-label="t('aiCenter.filterPrincipal')"
              class="filter-select"
              @update:model-value="onFilter"
            />
            <FpSelect
              v-model="toolFilter"
              :options="toolOptions"
              option-label="label"
              option-value="value"
              :aria-label="t('aiCenter.filterTool')"
              class="filter-select"
              @update:model-value="onFilter"
            />
          </div>
          <FpTable
            :rows="calls"
            :loading="callsLoading"
            :empty-text="t('aiCenter.noCalls')"
            striped-rows
          >
            <FpColumn :header="t('aiCenter.time')" style="width: 180px">
              <template #body="{ data }">
                <span class="mono">{{ data.created_at }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.tool')" style="width: 170px">
              <template #body="{ data }">
                <span class="mono">{{ data.tool }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.principal')" style="width: 150px">
              <template #body="{ data }">{{ principalName(data.principal_id) }}</template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.result')" style="width: 90px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.ok ? 'success' : 'danger'"
                  :value="data.ok ? t('aiCenter.ok') : t('aiCenter.failed')"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.duration')" style="width: 100px">
              <template #body="{ data }">
                <span class="mono">{{ data.duration_ms }} ms</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.arguments')" style="min-width: 220px">
              <template #body="{ data }">
                <code class="args" :title="data.arguments">{{ data.arguments }}</code>
                <div v-if="data.error" class="call-error">{{ data.error }}</div>
              </template>
            </FpColumn>
          </FpTable>
          <FpPagination
            v-if="total > pageSize"
            :first="first"
            :rows="pageSize"
            :total="total"
            :rows-per-page-options="[20, 50, 100]"
            @update:first="onFirst"
          />
        </div>
      </template>

      <!-- 面板内置助手（M-AI-3） -->
      <template #assistant>
        <AssistantChatPanel
          :enabled="assistantConfig?.enabled ?? false"
          :can-chat="canCreate"
          :can-configure="canConfigureAssistant"
          :runbooks="prompts"
          @open-approvals="tab = 'approvals'"
        />
        <AssistantSettingsPanel
          class="assistant-settings-block"
          :config="assistantConfig"
          :can-edit="canConfigureAssistant"
          @saved="onAssistantSaved"
        />
      </template>

      <!-- 工具目录 -->
      <template #tools>
        <div class="panel">
          <FpTable :rows="tools" :loading="toolsLoading" :empty-text="t('common.noData')" striped-rows>
            <FpColumn :header="t('aiCenter.tool')" style="width: 190px">
              <template #body="{ data }">
                <span class="mono">{{ data.name }}</span>
              </template>
            </FpColumn>
            <FpColumn field="title" :header="t('aiCenter.toolTitle')" style="width: 180px" />
            <FpColumn field="description" :header="t('aiCenter.description')" style="min-width: 260px" />
            <FpColumn :header="t('aiCenter.risk')" style="width: 90px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.risk === 'destructive' ? 'danger' : data.risk === 'write' ? 'warning' : 'info'"
                  :value="t(`aiCenter.risks.${data.risk}`)"
                />
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.permission')" style="width: 150px">
              <template #body="{ data }">
                <span class="mono">{{ data.permission }}</span>
              </template>
            </FpColumn>
            <FpColumn :header="t('aiCenter.allowed')" style="width: 100px">
              <template #body="{ data }">
                <FpTag
                  :severity="data.allowed ? 'success' : 'neutral'"
                  :value="data.allowed ? t('aiCenter.allowedYes') : t('aiCenter.allowedNo')"
                />
              </template>
            </FpColumn>
          </FpTable>
          <p class="muted tools-hint">{{ t('aiCenter.toolsHint') }}</p>

          <h3 class="section-title prompts-title">{{ t('aiCenter.promptsTitle') }}</h3>
          <p class="muted">{{ t('aiCenter.promptsHint') }}</p>
          <FpTable :rows="prompts" :loading="promptsQuery.loading.value" :empty-text="t('common.noData')" striped-rows>
            <FpColumn field="name" :header="t('aiCenter.name')" style="min-width: 180px">
              <template #body="{ data }">
                <code class="mono">{{ data.name }}</code>
              </template>
            </FpColumn>
            <FpColumn field="title" :header="t('aiCenter.toolTitle')" style="min-width: 140px" />
            <FpColumn field="description" :header="t('aiCenter.description')" style="min-width: 260px" />
            <FpColumn :header="t('aiCenter.promptArgs')" style="min-width: 140px">
              <template #body="{ data }">
                <span v-if="!data.arguments.length" class="muted">{{ t('aiCenter.promptNoArgs') }}</span>
                <code v-for="a in data.arguments" v-else :key="a.name" class="mono prompt-arg" :title="a.description"
                  >{{ a.name }}{{ a.required ? '*' : '' }}</code
                >
              </template>
            </FpColumn>
          </FpTable>
        </div>
      </template>
    </FpTabs>

    <!-- 签发凭证 -->
    <FpModal v-model="createVisible" :header="t('aiCenter.issueTitle')" size="md">
      <form class="form" @submit.prevent="submitCreate">
        <FpInput
          v-model="form.name"
          :label="t('aiCenter.name')"
          :placeholder="t('aiCenter.namePlaceholder')"
          :error="nameError"
          maxlength="64"
        />
        <FpNumber
          v-model="form.expiresInDays"
          :label="t('aiCenter.expiresInDays')"
          :min="1"
          :max="3650"
          :placeholder="t('aiCenter.never')"
        />
        <small class="muted">{{ t('aiCenter.expiresHint') }}</small>
        <FpSwitch v-model="form.allowWrite" :label="t('aiCenter.allowWrite')" />
        <small class="muted">{{ t('aiCenter.allowWriteHint') }}</small>
      </form>
      <template #footer>
        <FpButton variant="secondary" :label="t('common.cancel')" @click="createVisible = false" />
        <FpButton :label="t('aiCenter.issue')" :loading="creating" @click="submitCreate" />
      </template>
    </FpModal>

    <!-- 一次性令牌展示 -->
    <FpModal
      v-model="issuedVisible"
      :header="t('aiCenter.issuedTitle')"
      size="lg"
      :closable="false"
      :dismissable-mask="false"
    >
      <FpInlineMessage severity="warn">{{ t('aiCenter.tokenOnce') }}</FpInlineMessage>
      <div class="token-row">
        <code class="token">{{ issued?.token }}</code>
        <CopyButton :text="issued?.token" />
      </div>
      <McpSnippetBlock v-if="issued" :url="issuedUrl" :token="issued.token" />
      <template #footer>
        <FpButton :label="t('aiCenter.savedIt')" @click="closeIssued" />
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import CopyButton from '@/components/ui/CopyButton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import McpSnippetBlock from '@/components/aiCenter/McpSnippetBlock.vue'
import ApprovalsPanel from '@/components/aiCenter/ApprovalsPanel.vue'
import AssistantChatPanel from '@/components/aiCenter/AssistantChatPanel.vue'
import AssistantSettingsPanel from '@/components/aiCenter/AssistantSettingsPanel.vue'
import { getAssistantConfig, type AssistantConfigView } from '@/api/assistant'
import { listMcpPrompts, type McpPromptInfo } from '@/api/mcp'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { useCrudPage } from '@/composables/useCrudPage'
import { queryKeys } from '@/api/queryKeys'
import { hasPermission } from '@/directives/permission'
import { useAuthStore } from '@/stores/auth'
import {
  createPrincipal,
  listApprovals,
  listMcpTools,
  listPrincipals,
  listToolCalls,
  revokePrincipal,
  type ApprovalPage,
  type CreatePrincipalResponse,
  type PrincipalView,
  type ToolCallRecord,
  type ToolInfo,
} from '@/api/mcp'
import { mcpUrl, principalState } from '@/utils/mcpSnippets'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const auth = useAuthStore()
const queryClient = useQueryCacheClient()

const canCreate = computed(() => hasPermission(auth.role, 'mcp:create'))
// 与后端字段级授权一致：本人凭证凭 mcp:create 自助吊销；他人凭证需 mcp:delete
function canRevoke(p: PrincipalView): boolean {
  if (!p.active) return false
  if (hasPermission(auth.role, 'mcp:delete')) return true
  return p.owner === auth.username && hasPermission(auth.role, 'mcp:create')
}

const tab = ref('credentials')
// 待审批数（页签角标）：30 秒轮询，审批 / 拒绝后立即刷新
const pendingQuery = useApiQuery<ApprovalPage>(
  queryKeys.mcp.pendingApprovals(),
  () => listApprovals({ status: 'pending', page: 1, page_size: 1 }),
  { refetchInterval: 30_000 },
)
const pendingCount = computed(() => pendingQuery.data.value?.total ?? 0)

const tabItems = computed(() => [
  { value: 'credentials', label: t('aiCenter.tabs.credentials'), icon: 'oi oi-key' },
  { value: 'approvals', label: t('aiCenter.tabs.approvals'), icon: 'oi oi-check-circle', badge: pendingCount.value },
  { value: 'calls', label: t('aiCenter.tabs.calls'), icon: 'oi oi-list' },
  { value: 'tools', label: t('aiCenter.tabs.tools'), icon: 'oi oi-wrench' },
  { value: 'assistant', label: t('aiCenter.tabs.assistant'), icon: 'oi oi-sparkles' },
])

// ── 面板内置助手（M-AI-3）────────────────────────────
const canConfigureAssistant = computed(() => hasPermission(auth.role, 'settings:update'))
const assistantQuery = useApiQuery<AssistantConfigView>(queryKeys.assistant.config(), getAssistantConfig)
const assistantConfig = computed<AssistantConfigView | null>(() => assistantQuery.data.value ?? null)

const promptsQuery = useApiQuery<McpPromptInfo[]>(queryKeys.mcp.prompts(), listMcpPrompts)
const prompts = computed<McpPromptInfo[]>(() => promptsQuery.data.value ?? [])

function onAssistantSaved(cfg: AssistantConfigView) {
  queryClient.setQueryData(queryKeys.assistant.config(), cfg)
  // 助手会按需签发内置凭证，刷新凭证列表
  queryClient.invalidateQueries({ queryKey: queryKeys.mcp.principals() })
}

const origin = typeof window !== 'undefined' ? window.location.origin : ''
const endpointUrl = computed(() => mcpUrl(origin))

// ── 凭证 ─────────────────────────────────────────────
const principalsQuery = useApiQuery<PrincipalView[]>(queryKeys.mcp.principals(), listPrincipals)
const principals = computed<PrincipalView[]>(() => principalsQuery.data.value ?? [])
const principalsLoading = principalsQuery.loading

function stateSeverity(p: PrincipalView): 'success' | 'danger' | 'warning' {
  const s = principalState(p)
  return s === 'active' ? 'success' : s === 'revoked' ? 'danger' : 'warning'
}

function principalName(id: number): string {
  return principals.value.find((p) => p.id === id)?.name ?? `#${id}`
}

function confirmRevoke(p: PrincipalView) {
  confirmAction({
    header: t('aiCenter.revoke'),
    message: t('aiCenter.revokeConfirm', { name: p.name }),
    acceptLabel: t('aiCenter.revoke'),
    rejectLabel: t('common.cancel'),
    danger: true,
    accept: async () => {
      try {
        await revokePrincipal(p.id)
        toast.success(t('aiCenter.revoked'))
        queryClient.invalidateQueries({ queryKey: queryKeys.mcp.all })
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

// ── 签发 ─────────────────────────────────────────────
const createVisible = ref(false)
const creating = ref(false)
const form = reactive<{ name: string; expiresInDays: number | null; allowWrite: boolean }>({
  name: '',
  expiresInDays: 90,
  allowWrite: false,
})
const nameTouched = ref(false)
const nameError = computed(() =>
  nameTouched.value && !form.name.trim() ? t('aiCenter.nameRequired') : '',
)

const issuedVisible = ref(false)
const issued = ref<CreatePrincipalResponse | null>(null)
const issuedUrl = computed(() => mcpUrl(origin, issued.value?.endpoint))

function openCreate() {
  form.name = ''
  form.expiresInDays = 90
  form.allowWrite = false
  nameTouched.value = false
  createVisible.value = true
}

async function submitCreate() {
  nameTouched.value = true
  if (!form.name.trim() || creating.value) return
  creating.value = true
  try {
    const res = await createPrincipal({
      name: form.name.trim(),
      expires_in_days: form.expiresInDays ?? null,
      allow_write: form.allowWrite,
    })
    issued.value = res.data
    createVisible.value = false
    issuedVisible.value = true
    queryClient.invalidateQueries({ queryKey: queryKeys.mcp.principals() })
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    creating.value = false
  }
}

/** 关闭后立刻丢弃明文令牌，不留在内存/状态里 */
function closeIssued() {
  issuedVisible.value = false
  issued.value = null
}

// ── 调用记录 ─────────────────────────────────────────
const crud = useCrudPage({ pageSize: 20 })
const { total, first, pageSize, onFirst } = crud
const principalFilter = ref('')
const toolFilter = ref('')

const callsQuery = useApiQuery<ToolCallRecord[]>(
  () => queryKeys.mcp.calls(crud.currentPage.value, crud.pageSize.value, principalFilter.value, toolFilter.value),
  async () => {
    const res = await listToolCalls({
      principal_id: principalFilter.value ? Number(principalFilter.value) : null,
      tool: toolFilter.value || null,
      page: crud.currentPage.value,
      page_size: crud.pageSize.value,
    })
    crud.total.value = res.data.total
    return { data: res.data.data }
  },
  { keepPrevious: true },
)
const calls = computed<ToolCallRecord[]>(() => callsQuery.data.value ?? [])
const callsLoading = callsQuery.loading

function onFilter() {
  crud.currentPage.value = 1
}

function showCallsOf(id: number) {
  principalFilter.value = String(id)
  onFilter()
  tab.value = 'calls'
}

const principalOptions = computed(() => [
  { label: t('aiCenter.allPrincipals'), value: '' },
  ...principals.value.map((p) => ({ label: p.name, value: String(p.id) })),
])

// ── 工具目录 ─────────────────────────────────────────
const toolsQuery = useApiQuery<ToolInfo[]>(queryKeys.mcp.tools(), listMcpTools)
const tools = computed<ToolInfo[]>(() => toolsQuery.data.value ?? [])
const toolsLoading = toolsQuery.loading

const toolOptions = computed(() => [
  { label: t('aiCenter.allTools'), value: '' },
  ...tools.value.map((tool) => ({ label: tool.name, value: tool.name })),
])

function onApprovalsChanged() {
  void pendingQuery.refresh()
}

async function refreshAll() {
  await Promise.all([
    principalsQuery.refresh(),
    callsQuery.refresh(),
    toolsQuery.refresh(),
    pendingQuery.refresh(),
    assistantQuery.refresh(),
  ])
}
</script>

<style scoped>
.snippets,
.assistant-settings-block {
  margin-top: var(--fp-space-4);
}
.section-title {
  margin: 0 0 var(--fp-space-1);
  font-size: var(--fp-font-md, 15px);
}
.muted {
  color: var(--fp-text-muted);
  font-size: 13px;
  margin: 0 0 var(--fp-space-3);
}
.prompts-title {
  margin-top: var(--fp-space-5, 24px);
}
.prompt-arg + .prompt-arg {
  margin-left: var(--fp-space-1);
}
.tools-hint {
  margin-top: var(--fp-space-3);
}
.filters {
  display: flex;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-3);
  flex-wrap: wrap;
}
.filter-select {
  width: 200px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-1);
  align-items: center;
}
.args {
  display: block;
  max-width: 420px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}
.call-error {
  color: var(--fp-danger);
  font-size: 12px;
  margin-top: 2px;
}
.form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.token-row {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin: var(--fp-space-3) 0;
  padding: var(--fp-space-2) var(--fp-space-3);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius, 6px);
  background: var(--fp-bg-subtle, transparent);
}
.token {
  flex: 1;
  font-size: 13px;
  word-break: break-all;
}
</style>
