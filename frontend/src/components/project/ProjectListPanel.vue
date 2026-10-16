<template>
  <div class="panel">
    <FpTable
      :rows="rows"
      :loading="loading"
      :empty-text="t('common.noData')"
      :empty-desc="$slots.emptyAction ? t('pageDesc.emptyHint') : ''"
      striped-rows
    >
      <template v-if="$slots.emptyAction" #emptyAction>
        <slot name="emptyAction" />
      </template>
      <FpColumn field="name" :header="t('project.name')" />
      <FpColumn field="kind" :header="t('project.kind')" style="width: 100px">
        <template #body="{ data }">{{ kindLabel(data.kind, t) }}</template>
      </FpColumn>
      <FpColumn field="source" :header="t('project.source')" style="width: 120px">
        <template #body="{ data }">{{ sourceLabel(data.source, t) }}</template>
      </FpColumn>
      <FpColumn field="deploy_type" :header="t('project.deployType')" style="width: 110px">
        <template #body="{ data }">
          <FpTag
            :value="t('project.deployTypes.' + data.deploy_type)"
            :severity="data.deploy_type === 'webhook' ? 'info' : 'neutral'"
          />
        </template>
      </FpColumn>
      <FpColumn field="build_type" :header="t('project.buildType')" style="width: 120px">
        <template #body="{ data }">
          <FpTag
            :value="t('project.buildTypes.' + data.build_type)"
            :severity="data.build_type === 'dockerfile' ? 'info' : 'neutral'"
          />
        </template>
      </FpColumn>
      <FpColumn field="port" :header="t('project.port')" style="width: 80px">
        <template #body="{ data }">{{ data.port ?? '-' }}</template>
      </FpColumn>
      <FpColumn field="domain" :header="t('project.domain')" style="width: 160px">
        <template #body="{ data }">{{ data.domain ?? '-' }}</template>
      </FpColumn>
      <FpColumn :header="t('common.status')" style="width: 100px">
        <template #body="{ data }">
          <FpTag :value="statusLabel(data.status, t)" :severity="statusSeverity(data.status)" dot />
        </template>
      </FpColumn>
      <FpColumn :header="t('common.colActions')" style="width: 460px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'project:create', mode: 'view' }"
              variant="secondary"
              size="small"
              :loading="actingId === `deploy-${data.id}`"
              @click="emit('deploy', data)"
            >
              {{ t('project.deploy') }}
            </FpButton>
            <FpButton
              v-if="data.status !== 'running'"
              v-permission="{ perm: 'project:update', mode: 'view' }"
              variant="primary"
              size="small"
              :loading="actingId === `start-${data.id}`"
              @click="emit('start', data)"
            >
              {{ t('project.start') }}
            </FpButton>
            <FpButton
              v-else
              v-permission="{ perm: 'project:update', mode: 'view' }"
              variant="warning"
              size="small"
              @click="emit('stop', data)"
            >
              {{ t('project.stop') }}
            </FpButton>
            <FpButton variant="ghost" size="small" @click="emit('logs', data)">
              {{ t('project.logs') }}
            </FpButton>
            <FpActionMenu :items="rowActions(data)" />
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ProjectDto } from '@/api/generated'
import { kindLabel, sourceLabel, statusLabel, statusSeverity } from '@/utils/projectFormat'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpActionMenu, { type FpActionItem } from '@/components/ui/FpActionMenu.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * 项目列表（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 全站操作最密的一张表：**十类行操作**（部署 / 启动 / 停止 / 重启 / 健康探测 /
 * Webhook / 日志 / 联链 / 编辑 / 删除），每条的 `disabled` 条件都带业务语义——
 * 它们是本 spec 的重点（禁用条件写错不会报错，只会让用户对着点了没反应的按钮）：
 *
 * | 操作 | 禁用条件 | 语义 |
 * |------|---------|------|
 * | 启动 | `status === 'running'` | 已在跑的不能再启动 |
 * | 停止 | `status !== 'running'` | 没在跑的「停止」无意义 |
 * | 健康探测 | `!health_check_path` | 没配探测路径就没有可探测的目标 |
 * | Webhook | `source !== 'git'` | 本地目录没有可推送的仓库 |
 *
 * `actingId` 形如 `deploy-3`：**只锁当前行的当前按钮**（同一行有五个按钮，
 * 若只按行锁，用户点「部署」时「重启」也会被转圈）。故 loading 逐一绑定，
 * 「停止」没有 loading 绑定也是原实现口径（它的执行极快，且与启动互斥）。
 *
 * 权限、列宽、冻结操作列沿用原实现（拆分不改视觉与授权契约）。
 */
const props = withDefaults(
  defineProps<{
    rows: ProjectDto[]
    loading?: boolean
    /** 正在进行的行操作标签（`<动作>-<id>`；空串 = 空闲） */
    actingId?: string
  }>(),
  { loading: false, actingId: '' },
)

const emit = defineEmits<{
  deploy: [row: ProjectDto]
  start: [row: ProjectDto]
  stop: [row: ProjectDto]
  restart: [row: ProjectDto]
  health: [row: ProjectDto]
  webhook: [row: ProjectDto]
  logs: [row: ProjectDto]
  links: [row: ProjectDto]
  edit: [row: ProjectDto]
  remove: [row: ProjectDto]
}>()

const { t } = useI18n()

/** 行操作：部署 / 启停 / 日志 为按钮，其余收进「更多」 */
function rowActions(row: ProjectDto): FpActionItem[] {
  return [
    {
      key: 'restart',
      label: t('project.restart'),
      icon: 'oi oi-sync',
      perm: 'project:update',
      loading: props.actingId === `restart-${row.id}`,
      command: () => emit('restart', row),
    },
    {
      key: 'health',
      label: t('project.healthCheck'),
      icon: 'oi oi-heart',
      perm: 'project:update',
      disabled: !row.health_check_path,
      loading: props.actingId === `health-${row.id}`,
      command: () => emit('health', row),
    },
    {
      key: 'webhook',
      label: row.deploy_type === 'webhook' ? t('project.webhookReset') : t('project.webhookEnable'),
      icon: 'oi oi-link',
      perm: 'project:update',
      disabled: row.source !== 'git',
      command: () => emit('webhook', row),
    },
    {
      key: 'links',
      label: t('project.links'),
      icon: 'oi oi-database',
      perm: 'project:read',
      command: () => emit('links', row),
    },
    {
      key: 'edit',
      label: t('common.edit'),
      icon: 'oi oi-pencil',
      perm: 'project:update',
      separatorBefore: true,
      command: () => emit('edit', row),
    },
    {
      key: 'remove',
      label: t('common.delete'),
      icon: 'oi oi-trash',
      perm: 'project:delete',
      danger: true,
      command: () => emit('remove', row),
    },
  ]
}
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-1);
  flex-wrap: wrap;
}
</style>
