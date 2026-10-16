<template>
  <LayoutContent :title="t('backup.title')" :description="t('pageDesc.backups')" reload @reload="invalidate">
    <template #toolbar>
      <FpButton
        v-if="activeTab === 'local'"
        v-permission="{ perm: 'backup:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        :loading="creating"
        @click="onCreate"
      >
        {{ t('backup.create') }}
      </FpButton>
      <template v-else-if="activeTab === 'accounts'">
        <FpButton
          v-permission="{ perm: 'backup:create', mode: 'view' }"
          variant="ghost"
          icon="oi oi-cloud-download"
          @click="openRestoreDialog"
        >
          {{ t('backup.cloudRestore') }}
        </FpButton>
        <FpButton
          v-permission="{ perm: 'backup:create', mode: 'view' }"
          variant="primary"
          icon="oi oi-plus"
          @click="openAccountDialog()"
        >
          {{ t('backup.accountCreate') }}
        </FpButton>
      </template>
      <FpButton
        v-else
        v-permission="{ perm: 'backup:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="openJobDialog()"
      >
        {{ t('backup.jobCreate') }}
      </FpButton>
    </template>

    <FpTabs v-model="activeTab" :items="tabItems">
      <template #local>
        <LocalBackupPanel
          :rows="backups"
          :loading="loadingBackups"
          @download="onDownload"
          @restore="onRestore"
          @remove="onDelete"
        >
          <template #emptyAction>
            <FpButton v-permission="{ perm: 'backup:create', mode: 'view' }" variant="primary" icon="oi oi-plus" :loading="creating" @click="onCreate">{{ t('backup.create') }}</FpButton>
          </template>
        </LocalBackupPanel>
      </template>

      <template #accounts>
        <BackupAccountPanel
          :rows="accounts"
          :loading="loadingAccounts"
          :testing-id="testingId"
          @toggle="onToggleAccount"
          @test="onTestAccount"
          @edit="openAccountDialog"
          @remove="onDeleteAccount"
        >
          <template #emptyAction>
            <FpButton v-permission="{ perm: 'backup:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openAccountDialog()">{{ t('backup.accountCreate') }}</FpButton>
          </template>
        </BackupAccountPanel>
      </template>

      <template #jobs>
        <BackupJobPanel
          :rows="jobs"
          :loading="loadingJobs"
          :accounts="accounts"
          :running-id="runningJobId"
          @toggle="onToggleJob"
          @run="onRunJob"
          @edit="openJobDialog"
          @remove="onDeleteJob"
        >
          <template #emptyAction>
            <FpButton v-permission="{ perm: 'backup:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openJobDialog()">{{ t('backup.jobCreate') }}</FpButton>
          </template>
        </BackupJobPanel>
      </template>
    </FpTabs>

    <!-- 云端恢复：选账号 → 拉远端对象 → 一键还原（面板库 / 应用数据按 key 自动分流） -->
    <RestoreCloudDialog
      ref="restoreDialog"
      v-model:account-id="restoreAccountId"
      :account-options="accountSelectOptions"
      :objects="objects"
      :loading="objectsLoading"
      :restoring-key="restoringKey"
      @load="loadObjects"
      @restore="onRestoreFromCloud"
    />

    <!-- 账号创建/编辑（编辑时 config_json 留空表示不修改，后端不回显敏感配置） -->
    <BackupAccountDialog
      ref="accountDialog"
      :submitting="savingAccount"
      :type-options="accountTypeOptions"
      @submit="saveAccount"
    />

    <!-- 任务创建/编辑 -->
    <BackupJobDialog
      ref="jobDialog"
      :submitting="savingJob"
      :account-options="accountSelectOptions"
      @submit="saveJob"
    />
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { BackupAccountDto, BackupJobDto } from '@/api/generated'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import FpButton from '@/components/ui/FpButton.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import LocalBackupPanel from '@/components/backup/LocalBackupPanel.vue'
import BackupAccountPanel from '@/components/backup/BackupAccountPanel.vue'
import BackupJobPanel from '@/components/backup/BackupJobPanel.vue'
import RestoreCloudDialog from '@/components/backup/RestoreCloudDialog.vue'
import BackupAccountDialog from '@/components/backup/BackupAccountDialog.vue'
import BackupJobDialog from '@/components/backup/BackupJobDialog.vue'
import { useBackupLocal } from '@/composables/useBackupLocal'
import { useBackupAccounts } from '@/composables/useBackupAccounts'
import { useBackupJobs, type BackupJobFormPayload } from '@/composables/useBackupJobs'
import type { BackupAccountFormPayload } from '@/composables/useBackupAccounts'
import { accountTypeOptions as buildAccountTypeOptions } from '@/utils/backupFormat'

/**
 * 数据备份视图（Doc/23 §3.3 T2-4 巨型视图拆分第四站）。
 *
 * 拆分前 **905 行**、三个页签（本地备份 / 备份账号 / 计划任务）各自携带表格、
 * 弹窗与请求编排。本次沿用前三站（AppStoreView / WebsitesView / DockerView）
 * 已验证的四层拆法：
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `useBackupLocal` / `useBackupAccounts` / `useBackupJobs` | 三族请求编排、二次确认、表单载荷组装、远端对象 |
 * | 展示 | `components/backup/*`（6 个） | 三张表 + 三个弹窗，操作全部上抛 |
 * | 纯函数 | `utils/backupFormat.ts` | 状态映射、范围/范围标签、可还原判定、账号候选 |
 * | 装配 | 本文件 | 页签骨架 + composable ↔ 组件接线 |
 *
 * **与前三站的关键差异**：本站三族的**数据是互相引用的**——任务表要拿账号缓存
 * 译名字、任务弹窗与云端恢复共用同一份「启用账号」候选、云端恢复又要按账号拉
 * 远端对象。故本次没有把三族塞进一个大 composable，而是让引用关系**显式化**：
 * 账号候选由装配层从 `useBackupAccounts` 取出后传给另外两个消费点，
 * 「谁依赖谁」在接线处一眼可读，也不给数据层之间引入横向依赖。
 *
 * 行为契约不变：同一批 API、同一文案 key、同一确认分支与禁用条件、同一弹窗初值。
 */
const { t } = useI18n()

const tabItems: FpTabItem[] = [
  { value: 'local', label: t('backup.tabLocal') },
  { value: 'accounts', label: t('backup.tabAccounts') },
  { value: 'jobs', label: t('backup.tabJobs') },
]
const activeTab = ref('local')

/* ── 三族数据层 ── */

const {
  backups,
  loading: loadingBackups,
  creating,
  invalidateAll,
  create,
  download,
  restore,
  remove: removeBackup,
} = useBackupLocal()

const {
  accounts,
  loading: loadingAccounts,
  testingId,
  saving: savingAccount,
  beginEdit: beginAccountEdit,
  selectOptions: accountSelectOptions,
  save: saveAccountRow,
  toggle: toggleAccount,
  test: testAccount,
  remove: removeAccount,
  objects,
  objectsLoading,
  restoreAccountId,
  loadObjects,
  openRestore,
} = useBackupAccounts()

const {
  jobs,
  loading: loadingJobs,
  saving: savingJob,
  beginEdit: beginJobEdit,
  runningId: runningJobId,
  restoringKey,
  save: saveJobRow,
  toggle: toggleJob,
  run: runJob,
  remove: removeJob,
  confirmRestoreCloud,
} = useBackupJobs()

const invalidate = invalidateAll

/* ── 本地备份：行操作与新建 ── */

const onCreate = create
const onDownload = (filename: string) => void download(filename)
const onRestore = (filename: string) => restore(filename)
const onDelete = (filename: string) => removeBackup(filename)

/* ── 账号弹窗（打开时把编辑目标交给数据层，关弹窗由数据层回传决定） ── */

const accountDialog = ref<InstanceType<typeof BackupAccountDialog> | null>(null)
const accountTypeOptions = computed(() => buildAccountTypeOptions(t))

function openAccountDialog(row?: BackupAccountDto) {
  beginAccountEdit(row)
  accountDialog.value?.open(row)
}

async function saveAccount(payload: BackupAccountFormPayload) {
  if (await saveAccountRow(payload)) accountDialog.value?.close()
}

const onToggleAccount = (row: BackupAccountDto, enabled: boolean) => toggleAccount(row, enabled)
const onTestAccount = (row: BackupAccountDto) => void testAccount(row)
const onDeleteAccount = (row: BackupAccountDto) => removeAccount(row)

/* ── 云端恢复：账号初值 → 打开 → 拉对象 → 逐对象还原 ── */

const restoreDialog = ref<InstanceType<typeof RestoreCloudDialog> | null>(null)

function openRestoreDialog() {
  openRestore(accountSelectOptions.value[0]?.value ?? 0)
  restoreDialog.value?.open()
  if (restoreAccountId.value) void loadObjects()
}

function onRestoreFromCloud(key: string) {
  confirmRestoreCloud(restoreAccountId.value, key)
}

/* ── 任务弹窗与行操作 ── */

const jobDialog = ref<InstanceType<typeof BackupJobDialog> | null>(null)

function openJobDialog(row?: BackupJobDto) {
  beginJobEdit(row)
  jobDialog.value?.open(row)
}

async function saveJob(payload: BackupJobFormPayload) {
  if (await saveJobRow(payload)) jobDialog.value?.close()
}

const onToggleJob = (row: BackupJobDto, enabled: boolean) => toggleJob(row, enabled)
const onRunJob = (row: BackupJobDto) => runJob(row)
const onDeleteJob = (row: BackupJobDto) => removeJob(row)
</script>
