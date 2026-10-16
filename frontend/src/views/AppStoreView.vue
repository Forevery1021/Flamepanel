<template>
  <LayoutContent :title="t('nav.appStore')" :description="t('pageDesc.appStore')">
    <PageToolbar v-model="search" :search-placeholder="t('appStore.searchPlaceholder')">
      <template #actions>
        <FpButton
          v-permission="{ perm: 'app_store:create', mode: 'view' }"
          variant="ghost"
          icon="oi oi-folder-open"
          @click="showImportDialog = true"
        >
          {{ t('appStore.import') }}
        </FpButton>
      </template>
    </PageToolbar>

    <FpTabs v-model="activeTab" class="store-tabs" :items="tabItems">
      <template #store>
        <AppStoreGrid
          :packages="filteredPackages"
          :recommended="recommendedPackages"
          :installed-keys="installedKeys"
          :loading="loading"
          @install="openInstall"
        />
      </template>
      <template #installed>
        <InstalledAppsPanel
          :apps="installedApps"
          :loading="loadingInstalled"
          @uninstall="uninstall"
          @upgrade="upgradeTarget = $event"
          @logs="showLogs"
          @native-start="handleNativeStart"
          @native-stop="handleNativeStop"
          @native-restart="handleNativeRestart"
        />
      </template>
      <template #security>
        <StoreSecurityPanel
          :loading="securityLoading"
          :saving="securitySaving"
          :config="securityConfig"
          @save="onSaveSecurity"
          @reload="loadSecurity"
        />
      </template>
      <template #wasm>
        <AppStoreGrid
          :packages="wasmBuiltins"
          :loading="loadingWasm"
          :show-recommend="false"
          wasm
          @install="openInstall"
        />
      </template>
      <template #sources>
        <SourcesPanel
          :sources="sources"
          :loading="sourcesLoading"
          :syncing-source-id="syncingSourceId"
          @create="openSourceDialog"
          @sync="onSyncSource"
          @edit-key="openKeyDialog"
          @toggle="toggleSource"
          @remove="removeSource"
        />
      </template>
    </FpTabs>

    <AppInstallModal ref="installModal" @installed="loadInstalled" />

    <SourceDialogs
      v-model="sourceDialogVisible"
      v-model:key-visible="keyDialogVisible"
      :creating="creatingSource"
      :saving-key="savingKey"
      :source="keyDialogSource"
      @create="onCreateSource"
      @save-key="onSavePublicKey"
    />

    <FpModal v-model="logsVisible" :header="t('appStore.logs')" size="lg">
      <pre class="logs-view">{{ currentLogs }}</pre>
      <template #footer>
        <FpButton variant="primary" @click="logsVisible = false">
          {{ t('common.close') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="showImportDialog" :header="t('appStore.import')" size="md">
      <FpInput
        v-model="importPath"
        :label="t('appStore.importPath')"
        placeholder="/opt/flamepanel/apps/myapp"
      />
      <template #footer>
        <FpButton variant="ghost" @click="showImportDialog = false">
          {{ t('common.cancel') }}
        </FpButton>
        <FpButton variant="primary" @click="onImport">{{ t('appStore.import') }}</FpButton>
      </template>
    </FpModal>
    <AppUpgradeDialog v-model:app="upgradeTarget" @done="loadInstalled" />
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStoreCatalog } from '@/composables/useAppStoreCatalog'
import { useAppStoreSecurity } from '@/composables/useAppStoreSecurity'
import { useAppStoreSources } from '@/composables/useAppStoreSources'
import type { AppMetadata, AppSource, InstalledApp } from '@/api/appStore'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import FpModal from '@/components/ui/FpModal.vue'
import AppStoreGrid from '@/components/appStore/AppStoreGrid.vue'
import InstalledAppsPanel from '@/components/appStore/InstalledAppsPanel.vue'
import AppUpgradeDialog from '@/components/appStore/AppUpgradeDialog.vue'
import StoreSecurityPanel from '@/components/appStore/StoreSecurityPanel.vue'
import SourcesPanel from '@/components/appStore/SourcesPanel.vue'
import SourceDialogs from '@/components/appStore/SourceDialogs.vue'
import AppInstallModal from '@/components/appStore/AppInstallModal.vue'

/**
 * 应用商店视图（Doc/23 §3.3 T2-4 巨型视图拆分的首站）。
 *
 * 拆分前 1464 行，单文件承载 5 个页签 + 5 个弹窗 + 全部请求编排。本次按
 * 「列表区 / 表单弹窗 / 数据逻辑 composable」拆为四层：
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `composables/useAppStore{Catalog,Security,Sources}` + `useAppStoreInstall` | 请求编排、状态机、二次确认 |
 * | 展示 | `components/appStore/*`（5 个） | 纯装配，操作全部上抛 |
 * | 纯函数 | `utils/appStoreFormat.ts` | 卡片 / 表格 / 下拉三处共用的格式化映射 |
 * | 装配 | 本文件 | 页签骨架 + composable ↔ 组件的接线 |
 *
 * 行为契约不变（同一批 API、同一文案 key、同一确认分支）；拆分的收益是数据层
 * 进入 T1-3 覆盖率口径——`src/views/**` 被排除，`src/composables/**` 参与统计，
 * 新增单测由此真正看住这些逻辑（拆分前它们是「测试盲区 + 巨型文件」双重叠加）。
 */

const { t } = useI18n()

const {
  loading,
  loadingInstalled,
  loadingWasm,
  installedApps,
  wasmBuiltins,
  search,
  logsVisible,
  currentLogs,
  filteredPackages,
  recommendedPackages,
  installedKeys,
  loadPackages,
  loadInstalled,
  loadWasm,
  uninstall,
  handleNativeStart,
  handleNativeStop,
  handleNativeRestart,
  showLogs,
  doImport,
} = useAppStoreCatalog()

const {
  loading: securityLoading,
  saving: securitySaving,
  config: securityConfig,
  load: loadSecurity,
  activate: activateSecurity,
  save: saveSecurity,
} = useAppStoreSecurity()

/** 保存商店安全配置（载荷由展示层的编辑会话上抛） */
async function onSaveSecurity(payload: { signature_policy: string; registry_whitelist: string[] }) {
  await saveSecurity(payload)
}

const {
  sources,
  loading: sourcesLoading,
  syncingSourceId,
  creating: creatingSource,
  savingKey,
  keyDialogSource,
  loadSources,
  create: createSource,
  sync: syncSource,
  toggle: toggleSource,
  remove: removeSource,
  openKeyDialog: selectKeySource,
  savePublicKey,
} = useAppStoreSources()

// 弹窗可见性属 UI 状态（不入数据层）：成功后再关闭，失败保持打开以便修正输入
const sourceDialogVisible = ref(false)
/** 升级弹窗目标（三方合并预览）；null 关闭 */
const upgradeTarget = ref<InstalledApp | null>(null)
const keyDialogVisible = ref(false)

function openSourceDialog() {
  sourceDialogVisible.value = true
}

function openKeyDialog(row: AppSource) {
  selectKeySource(row)
  keyDialogVisible.value = true
}

async function onCreateSource(payload: {
  name: string
  source_type: string
  url: string
  public_key: string
}) {
  if (await createSource(payload)) sourceDialogVisible.value = false
}

async function onSavePublicKey(value: string) {
  if (await savePublicKey(value)) keyDialogVisible.value = false
}

const activeTab = ref('store')
const showImportDialog = ref(false)
const importPath = ref('')
const installModal = ref<InstanceType<typeof AppInstallModal> | null>(null)

const tabItems: FpTabItem[] = [
  { value: 'store', label: t('appStore.tabStore') },
  { value: 'installed', label: t('appStore.tabInstalled') },
  { value: 'sources', label: t('appStore.tabSources') },
  { value: 'security', label: t('appStore.tabSecurity') },
  { value: 'wasm', label: t('appStore.tabWasm') },
]

/** 安装弹窗入口：把待安装包交给弹窗（安装状态机在 `useAppStoreInstall` 内） */
function openInstall(pkg: AppMetadata) {
  void installModal.value?.open(pkg)
}

/** 远程源同步成功后刷新包列表（源索引变化直接影响商店页签内容） */
async function onSyncSource(row: AppSource) {
  if (await syncSource(row)) loadPackages()
}

async function onImport() {
  // 语义与原实现一致：成功后关弹窗（输入值保留，便于连续导入同目录下的多个包）
  if (await doImport(importPath.value)) showImportDialog.value = false
}

watch(activeTab, (tab) => {
  // 安全设置页懒加载（避免首屏多打一次配置接口）
  if (tab === 'security') activateSecurity()
})

onMounted(() => {
  loadPackages()
  loadInstalled()
  loadWasm()
  loadSources()
})
</script>

<style scoped>
.view-container {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.logs-view {
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  max-height: 420px;
  overflow-y: auto;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: var(--fp-font-mono);
}
</style>
