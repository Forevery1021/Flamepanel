<template>
  <LayoutContent :title="t('project.title')" :description="t('pageDesc.projects')" reload @reload="fetch">
    <template #toolbar>
      <PageToolbar v-model="search">
        <template #actions>
          <FpButton
            v-permission="{ perm: 'project:create', mode: 'view' }"
            variant="primary"
            icon="oi oi-plus"
            @click="formDialog?.open()"
          >
            {{ t('project.create') }}
          </FpButton>
        </template>
      </PageToolbar>
    </template>

    <ProjectListPanel
      :rows="filteredProjects"
      :loading="loading"
      :acting-id="actingId"
      @deploy="(row) => deploy(row, fetch)"
      @start="(row) => start(row, fetch)"
      @stop="(row) => stop(row, fetch)"
      @restart="(row) => restart(row, fetch)"
      @health="healthCheck"
      @webhook="(row) => toggleWebhook(row, fetch)"
      @logs="onOpenLogs"
      @links="openLinks"
      @edit="formDialog?.open($event)"
      @remove="(row) => remove(row, fetch)"
    >
      <template #emptyAction>
        <FpButton v-permission="{ perm: 'project:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="formDialog?.open()">{{ t('project.create') }}</FpButton>
      </template>
    </ProjectListPanel>

    <!-- 创建 / 编辑 -->
    <ProjectFormDialog
      ref="formDialog"
      :runtime-options="runtimeSelectOptions"
      :submitting="submitting"
      @create="onCreate"
      @update="(id, form) => onUpdate(id, form)"
    />

    <!-- Webhook 令牌展示（一次性） -->
    <WebhookDialog
      v-model="webhookVisible"
      :token="webhookToken"
      :url="webhookUrl"
      @copy="copyWebhookUrl"
    />

    <!-- 日志抽屉 -->
    <ProjectLogsDrawer
      v-model="logsVisible"
      :project="logsProject"
      :text="logsText"
      :lines="logLines"
      :loading="logsLoading"
      :fetching="logsFetching"
      :error="logsError"
      @refresh="refreshLogs"
    />

    <!-- 数据库联链抽屉（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式） -->
    <ProjectLinksDrawer
      v-model="linksVisible"
      :project="linksProject"
      :links="projectLinks"
      :linkable-options="linkableOptions"
      :form="linkForm"
      :loading="linksLoading"
      :error="linksError"
      :saving="linkSaving"
      :acting-id="linkActingId"
      @refresh="refreshLinks"
      @link="handleLink"
      @unlink="handleUnlink"
      @update:instance-id="(v) => (linkForm.db_instance_id = v)"
      @update:env-name="(v) => (linkForm.env_name = v ?? '')"
    />
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import PageToolbar from '@/components/ui/PageToolbar.vue'
import FpButton from '@/components/ui/FpButton.vue'
import ProjectListPanel from '@/components/project/ProjectListPanel.vue'
import ProjectFormDialog from '@/components/project/ProjectFormDialog.vue'
import WebhookDialog from '@/components/project/WebhookDialog.vue'
import ProjectLogsDrawer from '@/components/project/ProjectLogsDrawer.vue'
import ProjectLinksDrawer from '@/components/project/ProjectLinksDrawer.vue'
import { useProjectList } from '@/composables/useProjectList'
import { useProjectLifecycle } from '@/composables/useProjectLifecycle'
import { useProjectLogs } from '@/composables/useProjectLogs'
import { useProjectLinks } from '@/composables/useProjectLinks'
import type { ProjectFormState } from '@/utils/projectFormat'

/**
 * 项目管理视图（Doc/23 §3.3 T2-4 巨型视图拆分第六站）。
 *
 * 拆分前 **832 行**，是余下两个里最大的。它的形态与前五站都不同：**一个列表页 +
 * 两个抽屉 + 两个弹窗，但行操作多达十类**——部署 / 启动 / 停止 / 重启 / 健康探测 /
 * Webhook / 日志 / 联链 / 编辑 / 删除，每一条都带自己的确认分支或判成败口径。
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `useProjectList` | 列表 + 客户端搜索（名称/域名）+ 增改 + 运行时候选 |
 * | 数据 | `useProjectLifecycle` | 六类生命周期动作 + 删除 + Webhook 令牌与轮换 |
 * | 数据 | `useProjectLogs` | 日志惰性查询（打开才拉、关闭清目标） |
 * | 数据 | `useProjectLinks` | 联链两条查询 + 关联/解除 + 表单态 |
 * | 展示 | `components/project/*`（5 个） | 一张表 + 两个弹窗 + 两个抽屉，操作全部上抛 |
 * | 纯函数 | `utils/projectFormat.ts` | 字典映射、状态配色、载荷组装、日志拼接、Webhook 绝对地址 |
 * | 装配 | 本文件 | 列表接线 + 四个数据层的开合编排 |
 *
 * **本站的关键边界：`actingId` 是「行 × 动作」的二维锁**。同一行有五个并发按钮
 * （启动/停止/重启/健康/日志…），若按行加锁，用户点「部署」时「重启」会一起转圈；
 * 数据层因此用 `<动作>-<id>` 字符串标签而不是裸 id（原实现即如此，本次把它从
 * 视图内联状态提升为数据层契约）。**联链抽屉另有一把独立的锁**（`unlink-<linkId>`），
 * 两族的锁不共享——一个抽屉里的解除请求不该让列表里的按钮一起转圈。
 *
 * 行为契约不变：同一批 API、同一文案 key、同一禁用条件与确认分支、同一空值归一。
 */
const { t } = useI18n()

/* ── 列表：搜索 + 运行时候选（创建弹窗） ── */

const {
  filteredProjects,
  loading,
  search,
  fetch,
  submitting,
  create,
  update,
  runtimeSelectOptions,
} = useProjectList()

/* ── 生命周期：六类动作 + 删除 + Webhook ── */

const {
  actingId,
  webhookVisible,
  webhookToken,
  webhookUrl,
  deploy,
  start,
  stop,
  restart,
  healthCheck,
  remove,
  toggleWebhook,
  copyWebhookUrl,
} = useProjectLifecycle()

/* ── 创建 / 编辑弹窗：成功才关（失败让用户留在弹窗里改） ── */

const formDialog = ref<InstanceType<typeof ProjectFormDialog> | null>(null)

async function onCreate(form: ProjectFormState) {
  if (await create(form)) formDialog.value?.close()
}

async function onUpdate(id: number, form: ProjectFormState) {
  if (await update(id, form)) formDialog.value?.close()
}

/* ── 日志抽屉：打开才拉（惰性），关闭清目标 ── */

const {
  project: logsProject,
  lines: logLines,
  text: logsText,
  loading: logsLoading,
  fetching: logsFetching,
  error: logsError,
  refresh: refreshLogs,
  open: openLogs,
  reset: resetLogs,
} = useProjectLogs()

/** 日志抽屉可见性（数据层管数据，开合属 UI 状态） */
const logsVisible = ref(false)

function onOpenLogs(row: Parameters<typeof openLogs>[0]) {
  openLogs(row)
  logsVisible.value = true
}

watch(logsVisible, (visible) => {
  // 关闭时清空目标：再打开另一个项目时不残留上一份日志（避免一屏旧数据先渲染）
  if (!visible) resetLogs()
})

/* ── 联链抽屉 ── */

const {
  project: linksProject,
  links: projectLinks,
  linkableOptions,
  loading: linksLoading,
  error: linksError,
  saving: linkSaving,
  actingId: linkActingId,
  form: linkForm,
  refresh: refreshLinks,
  open: openLinksData,
  reset: resetLinks,
  link: handleLink,
  unlink: handleUnlink,
} = useProjectLinks()

const linksVisible = ref(false)

function openLinks(row: Parameters<typeof openLinksData>[0]) {
  openLinksData(row)
  linksVisible.value = true
}

watch(linksVisible, (visible) => {
  if (!visible) resetLinks()
})
</script>

<style scoped>
:deep(.panel) {
  width: 100%;
}
</style>
