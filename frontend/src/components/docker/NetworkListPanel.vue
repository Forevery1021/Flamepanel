<template>
  <div class="panel">
    <div class="toolbar">
      <FpButton
        v-permission="{ perm: 'docker:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="emit('create')"
        >{{ t('docker.createNetwork') }}</FpButton
      >
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
    >
      <FpColumn field="name" :header="t('docker.name')" style="min-width: 140px" />
      <FpColumn field="driver" :header="t('docker.driver')" style="width: 100px" />
      <FpColumn field="scope" :header="t('docker.scope')" style="width: 90px" />
      <FpColumn :header="t('docker.connectedContainers')" style="min-width: 180px">
        <template #body="{ data }">
          <span v-if="data.containers && data.containers.length">
            <FpTag v-for="c in data.containers" :key="c.name" severity="neutral" class="mr-1">
              {{ c.name }}<span v-if="c.ipv4_address"> ({{ c.ipv4_address }})</span>
            </FpTag>
          </span>
          <span v-else class="text-muted">—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('docker.actions')" style="width: 260px" frozen align-frozen="right">
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton variant="ghost" icon="oi oi-link" @click="emit('connect', data)">{{
              t('docker.connect')
            }}</FpButton>
            <FpButton variant="ghost" icon="oi oi-minus-circle" @click="emit('disconnect', data)">{{
              t('docker.disconnect')
            }}</FpButton>
            <FpButton
              v-permission="{ perm: 'docker:delete', mode: 'view' }"
              variant="danger"
              icon="oi oi-trash"
              @click="emit('remove', data)"
              >{{ t('docker.remove') }}</FpButton
            >
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { DockerNetwork } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * Docker 网络列表（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 「已连接容器」列在**无连接**时渲染破折号占位（`—`）而非空单元格：
 * 空着会让人怀疑「是不是没加载出来」，破折号明确表达「无」。连接/断开两个动作
 * 不设权限门槛（原实现如此——它们是网络自身的拓扑操作，不涉及容器生命周期）。
 */
withDefaults(
  defineProps<{
    rows: DockerNetwork[]
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  create: []
  connect: [row: DockerNetwork]
  disconnect: [row: DockerNetwork]
  remove: [row: DockerNetwork]
  prune: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
  flex-wrap: wrap;
}
.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.mr-1 {
  margin-right: 4px;
}
</style>
