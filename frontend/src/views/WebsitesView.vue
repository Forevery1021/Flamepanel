<template>
  <LayoutContent :title="t('nav.websites')" :description="t('pageDesc.websites')">
    <FpTabs v-model="activeTab" class="site-tabs" :items="tabItems">
      <template #websites>
        <PageToolbar v-model="searchText">
          <template #left>
            <FpSelect
              v-model="statusFilter"
              :options="statusOptions"
              option-label="label"
              option-value="value"
              show-clear
              class="w-[160px]"
            />
          </template>
          <template #actions>
            <FpButton
              v-permission="{ perm: 'website:create', mode: 'view' }"
              variant="primary"
              icon="oi oi-plus"
              @click="dialogs?.openCreate()"
            >
              {{ t('website.create') }}
            </FpButton>
          </template>
        </PageToolbar>

        <WebsiteListPanel
          :rows="filteredWebsites"
          :loading="loading"
          :error="websitesError"
          :total="total"
          :first="first"
          :page-size="pageSize"
          :deploying-id="deployingId"
          @edit="dialogs?.openEdit($event)"
          @switch-engine="onOpenSwitchEngine"
          @deploy="deploy"
          @bind="onOpenBind"
          @rewrite="onOpenRewrite"
          @delete="confirmDelete"
          @retry="fetch"
          @update:first="onFirst"
        >
          <template #emptyAction>
            <FpButton
              v-permission="{ perm: 'website:create', mode: 'view' }"
              variant="primary"
              icon="oi oi-plus"
              @click="dialogs?.openCreate()"
              >{{ t('website.create') }}</FpButton
            >
          </template>
        </WebsiteListPanel>

        <WebsiteDialogs
          ref="dialogs"
          :engines="engines"
          :php-runtime-options="phpRuntimeOptions"
          :project-options="projectOptions"
          :rewrite-preset-options="bindRewritePresetOptions"
          :submitting="submitting"
          :binding="bindingSubmitting"
          :rewriting="rewriteSubmitting"
          @create="onCreate"
          @update="onUpdate"
          @switch-engine="onSwitchEngine"
          @bind="onBind"
          @apply-rewrite="onApplyRewrite"
          @clear-rewrite="onClearRewrite"
        />
      </template>

      <template #certificates>
        <PageToolbar :searchable="false">
          <template #left>
            <FpButton variant="primary" icon="oi oi-lock-locked" @click="certDialogs?.openAcme()">
              {{ t('certificate.issueAcme') }}
            </FpButton>
            <FpButton variant="ghost" @click="certDialogs?.openSelf()">{{
              t('certificate.issueSelfSigned')
            }}</FpButton>
            <FpButton variant="ghost" @click="certDialogs?.openUpload()">{{
              t('certificate.upload')
            }}</FpButton>
          </template>
        </PageToolbar>

        <CertificateListPanel
          :certificates="certs"
          :loading="certsLoading"
          :error="certsError"
          :renewing-id="renewingId"
          @retry="fetchCerts"
          @bind="certDialogs?.openBind($event)"
          @unbind="unbindCert"
          @renew="renewCert"
          @delete="confirmDeleteCert"
          @toggle-auto-renew="toggleAutoRenew"
          @history="openCertHistory"
        >
          <template #emptyAction>
            <FpButton variant="primary" icon="oi oi-lock-locked" @click="certDialogs?.openAcme()">{{
              t('certificate.issueAcme')
            }}</FpButton>
          </template>
        </CertificateListPanel>

        <CertificateRenewalDrawer
          v-model="certHistoryVisible"
          :certificate="certHistoryCert"
          :history="certHistory"
          :loading="certHistoryFetching && !certHistory"
          :fetching="certHistoryFetching"
          :error="certHistoryError"
          @refresh="refreshCertHistory"
        />

        <CertificateDialogs
          ref="certDialogs"
          :submitting="certSubmitting"
          :dns-accounts="dnsAccounts"
          @required="toast.error(t('common.required'))"
          @acme="onIssueAcme"
          @self-signed="onIssueSelf"
          @upload="onUploadCert"
          @bind="onBindCert"
        />
      </template>
    </FpTabs>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import { useFpToast } from '@/components/ui/FpToast'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import type { Website } from '@/types'
import {
  rewritePresetOptions as buildRewritePresetOptions,
  siteStatusOptions,
} from '@/utils/websiteFormat'
import { useWebsiteList, type WebsiteFormPayload } from '@/composables/useWebsiteList'
import { useWebsiteBinding, type WebsiteBindPayload } from '@/composables/useWebsiteBinding'
import { useWebsiteRewrite } from '@/composables/useWebsiteRewrite'
import { useWebsiteCertificates } from '@/composables/useWebsiteCertificates'
import FpTabs from '@/components/ui/FpTabs.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import WebsiteListPanel from '@/components/website/WebsiteListPanel.vue'
import WebsiteDialogs from '@/components/website/WebsiteDialogs.vue'
import CertificateListPanel from '@/components/website/CertificateListPanel.vue'
import CertificateDialogs from '@/components/website/CertificateDialogs.vue'
import CertificateRenewalDrawer from '@/components/website/CertificateRenewalDrawer.vue'

/**
 * 网站管理视图（Doc/23 §3.3 T2-4 巨型视图拆分第二站）。
 *
 * 拆分前 1181 行、两个页签（网站 / SSL 证书）各自携带整套表格 + 弹窗 + 请求编排。
 * 本次按 AppStoreView（第一步）已验证的四层拆法处理：
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `useWebsiteList` / `useWebsiteBinding` / `useWebsiteRewrite` / `useWebsiteCertificates` | 请求编排、分页与筛选、二次确认、引擎清单与续签态 |
 * | 展示 | `components/website/*`（4 个） | 纯装配，操作全部上抛 |
 * | 纯函数 | `utils/websiteFormat.ts` | 域名/时间/证书三态/下拉候选四处共用的映射 |
 * | 装配 | 本文件 | 页签骨架 + composable ↔ 组件接线 |
 *
 * 行为契约不变（同一批 API、同一文案 key、同一确认分支）；拆分的收益是数据层进入
 * T1-3 覆盖率口径——`src/views/**` 被排除，`src/composables/**` 参与统计。
 *
 * **弹窗编排在本层**（与第一步的 AppStoreView 同构）：`openXxx` 前若需服务端数据
 * （引擎清单 / 绑定下拉 / 伪静态状态），由本层先 `await` 再打开；「成功才关」的
 * 判定也留在这里——弹窗组件只管展示与编辑，数据层只管请求。
 */
const { t } = useI18n()
const toast = useFpToast()

const {
  filteredWebsites,
  loading,
  error: websitesError,
  total,
  first,
  pageSize,
  onFirst,
  fetch,
  search: searchText,
  statusFilter,
  submitting,
  create,
  update,
  remove: confirmDelete,
  engines,
  loadEngines,
  switchEngine,
  deployingId,
  deploy,
} = useWebsiteList()

const {
  phpRuntimeOptions,
  projectOptions,
  submitting: bindingSubmitting,
  prepare: prepareBinding,
  bind,
} = useWebsiteBinding()

const rewrite = useWebsiteRewrite()
const {
  status: rewriteStatus,
  submitting: rewriteSubmitting,
  prepare: prepareRewrite,
  apply: applyRewrite,
  clear: clearRewrite,
} = rewrite

const {
  certificates: certs,
  loading: certsLoading,
  error: certsError,
  submitting: certSubmitting,
  renewingId,
  dnsAccounts,
  fetch: fetchCerts,
  loadDnsAccounts,
  issueAcmeCert,
  issueSelfSignedCert,
  upload: uploadCert,
  bind: bindCert,
  renew: renewCert,
  toggleAutoRenew,
  unbind: unbindCert,
  remove: confirmDeleteCert,
  historyVisible: certHistoryVisible,
  historyCert: certHistoryCert,
  history: certHistory,
  historyFetching: certHistoryFetching,
  historyError: certHistoryError,
  openHistory: openCertHistory,
  refreshHistory: refreshCertHistory,
} = useWebsiteCertificates()

const dialogs = ref<InstanceType<typeof WebsiteDialogs> | null>(null)
const certDialogs = ref<InstanceType<typeof CertificateDialogs> | null>(null)

const tabItems: FpTabItem[] = [
  { value: 'websites', label: t('website.title') },
  { value: 'certificates', label: t('certificate.title') },
]
const activeTab = ref('websites')

const statusOptions = computed(() => siteStatusOptions(t))
/** 绑定弹窗的伪静态候选：本地常量（不依赖该站点已有套用状态） */
const bindRewritePresetOptions = buildRewritePresetOptions()

/* ── 网站页签：写操作（成功才关弹窗） ── */

async function onCreate(payload: WebsiteFormPayload) {
  if (await create(payload)) dialogs.value?.closeCreate()
}

async function onUpdate(id: number, payload: WebsiteFormPayload) {
  if (await update(id, payload)) dialogs.value?.closeEdit()
}

/** 切换引擎：清单懒加载完成后再开弹窗（下拉需要有候选） */
async function onOpenSwitchEngine(row: Website) {
  await loadEngines()
  dialogs.value?.openSwitchEngine(row)
}

async function onSwitchEngine(id: number, engine: string) {
  if (await switchEngine(id, engine)) dialogs.value?.closeSwitchEngine()
}

/* ── 绑定：先备齐两个下拉的数据源，再开弹窗 ── */

async function onOpenBind(row: Website) {
  await prepareBinding()
  dialogs.value?.openBind(row)
}

async function onBind(id: number, payload: WebsiteBindPayload) {
  if (await bind(id, payload)) {
    dialogs.value?.closeBind()
    await fetch()
  }
}

/* ── 伪静态：先取服务端状态（模板清单以接口 supported 为权威） ── */

async function onOpenRewrite(row: Website) {
  // 先取服务端状态（含模板清单 supported），再以该状态为初值打开弹窗
  const status = await prepareRewrite(row.id)
  // 候选清单：接口 supported 优先（后端为模板库单一事实来源），未返回时弹窗内部回落常量
  dialogs.value?.openRewrite(row, status, rewrite.presetOptions.value)
}

async function onApplyRewrite(id: number, preset: string) {
  if (await applyRewrite(id, preset)) {
    dialogs.value?.syncRewriteStatus(rewriteStatus.value)
    dialogs.value?.closeRewrite()
    await fetch()
  }
}

async function onClearRewrite(id: number) {
  if (await clearRewrite(id)) {
    dialogs.value?.syncRewriteStatus(rewriteStatus.value)
    dialogs.value?.closeRewrite()
    await fetch()
  }
}

/* ── 证书页签：签发与绑定（成功才关弹窗） ── */

async function onIssueAcme(payload: {
  domains: string[]
  dns01: boolean
  webroot?: string
  dns_provider_account_id?: number
}) {
  if (await issueAcmeCert(payload)) certDialogs.value?.closeAcme()
}

async function onIssueSelf(domains: string[]) {
  if (await issueSelfSignedCert(domains)) certDialogs.value?.closeSelf()
}

async function onUploadCert(payload: { domains: string[]; cert_pem: string; key_pem: string }) {
  if (await uploadCert(payload)) certDialogs.value?.closeUpload()
}

async function onBindCert(id: number, websiteId: number | null) {
  if (await bindCert(id, websiteId)) certDialogs.value?.closeBind()
}

/* ── 生命周期 ── */

watch(activeTab, (tab) => {
  if (tab === 'certificates') {
    void fetchCerts()
    // DNS 厂商账号仅签发弹窗需要；随页签一起备好，失败静默（HTTP-01 路径不受影响）
    void loadDnsAccounts()
  }
})

onMounted(() => {
  void fetch()
})
</script>

<style scoped>
.site-tabs {
  width: 100%;
}
</style>
