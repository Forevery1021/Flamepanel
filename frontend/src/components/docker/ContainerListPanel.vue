<template>
  <div class="panel">
    <div class="toolbar">
      <div class="search-wrap">
        <FpInput v-model="search" :placeholder="t('docker.searchPlaceholder')" />
      </div>
      <FpButton
        v-permission="{ perm: 'docker:delete', mode: 'view' }"
        variant="danger"
        plain
        icon="oi oi-trash"
        @click="emit('prune')"
      >
        {{ t('docker.prune') }}
      </FpButton>
    </div>
    <FpTable
      :rows="rows"
      :loading="loading"
      :paginator="false"
      :empty-text="t('common.noData')"
      striped-rows
      virtual
      virtual-scroll-height="560px"
    >
      <FpColumn field="name" :header="t('docker.name')" style="min-width: 140px" />
      <FpColumn field="id" :header="t('docker.containerId')" style="width: 120px">
        <template #body="{ data }">
          <span v-tooltip="data.id" class="cell-truncate">{{ data.id }}</span>
        </template>
      </FpColumn>
      <FpColumn field="image" :header="t('docker.image')" style="min-width: 140px">
        <template #body="{ data }">
          <span v-tooltip="data.image" class="cell-truncate">{{ data.image }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('docker.status')" style="width: 110px">
        <template #body="{ data }">
          <FpTag
            :severity="containerStatusType(data.status)"
            :value="data.status"
            :dot="data.status === 'running'"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('docker.actions')" style="width: 220px" frozen align-frozen="right">
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-if="data.status !== 'running'"
              v-permission="{ perm: 'docker:start', mode: 'view' }"
              variant="ghost"
              icon="oi oi-play-circle"
              @click="emit('start', data)"
              >{{ t('docker.start') }}</FpButton
            >
            <FpButton
              v-else
              v-permission="{ perm: 'docker:stop', mode: 'view' }"
              variant="ghost"
              icon="oi oi-stop-circle"
              @click="emit('stop', data)"
              >{{ t('docker.stop') }}</FpButton
            >
            <FpButton variant="ghost" icon="oi oi-terminal" @click="emit('logs', data)">{{
              t('docker.logs')
            }}</FpButton>
            <FpActionMenu :items="rowActions(data)" />
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { containerStatusType } from '@/utils/dockerFormat'
import type { DockerContainer } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpActionMenu, { type FpActionItem } from '@/components/ui/FpActionMenu.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * Docker 容器列表（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * **纯展示 + 事件上抛**：九类行内操作（启停/重启/删除/强停/暂停恢复/日志/统计/
 * 详情/重命名）与工具栏的搜索、清理全部交由父级编排（`useDockerContainers`）。
 * 表格自身不持有请求状态，搜索框也只是本地 model（上抛 `update:search`）。
 *
 * 操作列的**禁用条件即状态机**（running 才能停、非 running 且非 paused 才能暂停等），
 * 这些布尔表达式与后端约束一一对应，是本组件最需要被测的部分；列宽与冻结列沿用
 * 原实现（拆分不改视觉契约）。
 */
withDefaults(
  defineProps<{
    rows: DockerContainer[]
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  start: [row: DockerContainer]
  stop: [row: DockerContainer]
  restart: [row: DockerContainer]
  remove: [row: DockerContainer]
  kill: [row: DockerContainer]
  pause: [row: DockerContainer]
  logs: [row: DockerContainer]
  stats: [row: DockerContainer]
  detail: [row: DockerContainer]
  rename: [row: DockerContainer]
  'export-compose': [row: DockerContainer]
  prune: []
}>()

/** 低频操作收进「更多」菜单；主操作（启/停、日志）保留为按钮 */
function rowActions(row: DockerContainer): FpActionItem[] {
  const running = row.status === 'running'
  const paused = row.status === 'paused'
  return [
    {
      key: 'restart',
      label: t('docker.restart'),
      icon: 'oi oi-sync',
      perm: 'docker:start',
      disabled: !running,
      command: () => emit('restart', row),
    },
    {
      key: 'pause',
      label: paused ? t('docker.unpause') : t('docker.pause'),
      icon: 'oi oi-pause-circle',
      perm: 'docker:start',
      disabled: !running && !paused,
      command: () => emit('pause', row),
    },
    {
      key: 'stats',
      label: t('docker.stats'),
      icon: 'oi oi-gauge',
      command: () => emit('stats', row),
    },
    {
      key: 'detail',
      label: t('docker.inspect'),
      icon: 'oi oi-eye',
      command: () => emit('detail', row),
    },
    {
      key: 'export-compose',
      label: t('docker.exportCompose'),
      icon: 'oi oi-file-export',
      command: () => emit('export-compose', row),
    },
    {
      key: 'rename',
      label: t('docker.rename'),
      icon: 'oi oi-pencil',
      perm: 'docker:update',
      command: () => emit('rename', row),
    },
    {
      key: 'kill',
      label: t('docker.kill'),
      icon: 'oi oi-bolt',
      perm: 'docker:start',
      disabled: !running,
      separatorBefore: true,
      command: () => emit('kill', row),
    },
    {
      key: 'remove',
      label: t('docker.remove'),
      icon: 'oi oi-trash',
      perm: 'docker:delete',
      danger: true,
      command: () => emit('remove', row),
    },
  ]
}

const { t } = useI18n()

/**
 * 搜索词走 `defineModel` 双向绑定：父级持有唯一数据源（触发过滤与重置），
 * 本组件只提供输入控件——避免「组件内再存一份搜索词」造成的两份真相。
 */
const search = defineModel<string>('search', { default: '' })
</script>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
  flex-wrap: wrap;
}
.search-wrap {
  flex: 1;
  max-width: 320px;
  min-width: 220px;
}
.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.cell-truncate {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
