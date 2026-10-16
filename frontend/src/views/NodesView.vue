<template>
  <LayoutContent :title="t('node.title')" :description="t('pageDesc.nodes')" reload @reload="fetch">
    <template #toolbar>
      <FpButton v-permission="{ perm: 'node:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="formDialog?.open()">
        {{ t('node.register') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="ghost" @click="openExecModal">
        {{ t('node.remoteExec') }}
      </FpButton>
      <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="ghost" @click="openBatchModal(nodes)">
        {{ t('node.batchExec') }}
      </FpButton>
    </template>

    <NodeListPanel
      :rows="nodes"
      :loading="loading"
      :total="total"
      :first="first"
      :page-size="pageSize"
      @edit="onOpenEdit"
      @exec="openExecForNode"
      @files="openFilesForNode"
      @monitor="openMonitor"
      @terminal="openTerminal"
      @remove="(row) => remove(row, fetch)"
      @update:first="onFirst"
    >
      <template #emptyAction>
        <FpButton v-permission="{ perm: 'node:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="formDialog?.open()">{{ t('node.register') }}</FpButton>
      </template>
    </NodeListPanel>

    <!-- 注册 / 编辑 -->
    <NodeFormDialog
      ref="formDialog"
      :submitting="submitting"
      @create="onCreate"
      @update="(id, form) => onUpdate(id, form)"
    />

    <!-- 远程命令 -->
    <NodeExecDialog
      v-model="execVisible"
      v-model:command="execCommand"
      :node-name="execNodeName"
      :loading="execLoading"
      :result="execResult"
      :error="execErrors.command"
      @run="run"
    />

    <!-- 批量命令 -->
    <NodeBatchExecDialog
      v-model="batchVisible"
      v-model:command="batchCommand"
      v-model:selected-ids="batchSelectedIds"
      :nodes="nodes"
      :loading="batchLoading"
      :result="batchResult"
      @run="runBatch"
    />

    <!-- 远程文件 -->
    <NodeFilesDialog
      v-model="filesVisible"
      v-model:path="filesPath"
      v-model:upload-name="filesUploadName"
      v-model:upload-content="filesUploadContent"
      :node-name="filesNodeName"
      :entries="filesEntries"
      :uploading="filesUploading"
      @enter="filesEnter"
      @up="filesGoUp"
      @load="filesLoad"
      @download="filesDownload"
      @upload="filesUpload"
    />
  </LayoutContent>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpButton from '@/components/ui/FpButton.vue'
import NodeListPanel from '@/components/node/NodeListPanel.vue'
import NodeFormDialog from '@/components/node/NodeFormDialog.vue'
import NodeExecDialog from '@/components/node/NodeExecDialog.vue'
import NodeBatchExecDialog from '@/components/node/NodeBatchExecDialog.vue'
import NodeFilesDialog from '@/components/node/NodeFilesDialog.vue'
import { useNodeList } from '@/composables/useNodeList'
import { useNodeCrud } from '@/composables/useNodeCrud'
import { useNodeRemoteExec } from '@/composables/useNodeRemoteExec'
import { useNodeFiles } from '@/composables/useNodeFiles'
import type { EnrichedNode, NodeFormState } from '@/utils/nodeFormat'

/**
 * 服务器节点视图（Doc/23 §3.3 T2-4 巨型视图拆分第七站，也是该队列的**收尾站**）。
 *
 * 拆分前 **824 行**，是余下最大的一个。它的形态与前六站都不同——**一张表 +
 * 四个弹窗**，但数据流是「**两段式**」的：
 *
 * 1. 列表接口只给「注册时填的期望状态」与一份可能过期的 `metrics_json` 快照；
 * 2. 真正在线与否、实时指标如何，靠列表每次加载完成后**逐节点并发查
 *    `/status` + `/metrics` 并写回同一份查询缓存**。
 *
 * 所以 824 行里最容易写错的地方不是某个弹窗，而是「**谁在什么时候覆盖谁**」：
 * 首屏、翻页、10s 轮询、页面切回四种时机都会触发覆盖，而写回目标必须是
 * 「查询缓存里的当前页」（否则轮询会把状态写到别的页上去）。
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `useNodeList` | 列表 + 分页 + **两段式状态覆盖** + 页面切回补刷 |
 * | 数据 | `useNodeCrud` | 注册 / 编辑 / 删除（二次确认） |
 * | 数据 | `useNodeRemoteExec` | 远程命令（失败落结果区）+ 批量命令（全选当前页） |
 * | 数据 | `useNodeFiles` | 远程文件：浏览 / 进入 / 上跳 / 下载 / 上传 |
 * | 展示 | `components/node/*`（5 个） | 一张表 + 四个弹窗，操作全部上抛 |
 * | 纯函数 | `utils/nodeFormat.ts` | 指标解析与摘要、取色、路径拼接、base64 编解码、表单校验与更新体组装 |
 * | 装配 | 本文件 | 列表接线 + 四个数据层的开合编排 |
 *
 * 行为契约不变：同一批 API、同一文案 key、同一轮询间隔与可见性暂停语义、
 * 同一弹窗初值与确认分支。
 */
const { t } = useI18n()

/* ── 列表：两段式状态覆盖 ── */

const { nodes, loading, total, first, pageSize, onFirst, fetch } = useNodeList()

/* ── 增删改 ── */

const { submitting, create, save, remove } = useNodeCrud()

const formDialog = ref<InstanceType<typeof NodeFormDialog> | null>(null)

/** 行内「编辑」：把整行交给弹窗（弹窗自己取四个可编辑字段） */
function onOpenEdit(row: EnrichedNode) {
  formDialog.value?.open(row)
}

async function onCreate(form: NodeFormState) {
  if (await create(form, fetch)) formDialog.value?.close()
}

async function onUpdate(id: number, form: NodeFormState) {
  if (await save(id, form, fetch)) formDialog.value?.close()
}

/* ── 远程命令 / 批量命令 ── */

const {
  execVisible,
  execNodeName,
  execCommand,
  execErrors,
  execLoading,
  execResult,
  openExecModal,
  openExecForNode,
  run,
  batchVisible,
  batchCommand,
  batchSelectedIds,
  batchLoading,
  batchResult,
  openBatchModal,
  runBatch,
} = useNodeRemoteExec()

/* ── 远程文件 ── */

const {
  visible: filesVisible,
  nodeName: filesNodeName,
  path: filesPath,
  entries: filesEntries,
  uploading: filesUploading,
  uploadName: filesUploadName,
  uploadContent: filesUploadContent,
  openForNode: openFilesForNode,
  load: filesLoad,
  enter: filesEnter,
  goUp: filesGoUp,
  download: filesDownload,
  upload: filesUpload,
} = useNodeFiles()

/** Doc/30 M-P：「监控」→ 仪表盘切到该节点（`?node=`，指标来自 Agent 心跳并入的 MetricsHub） */
const router = useRouter()
function openMonitor(row: { id: number }) {
  router.push({ path: '/dashboard', query: { node: String(row.id) } })
}

/** Agent Edge：「远程终端」→ 独立页面（xterm 需要整屏空间，不做弹窗） */
function openTerminal(row: { id: number; name: string }) {
  router.push({ name: 'NodeTerminal', params: { id: String(row.id) }, query: { name: row.name } })
}
</script>

<style scoped>
:deep(.panel) {
  width: 100%;
}
</style>
