<template>
  <div class="panel">
    <div class="toolbar">
      <FpButton
        v-permission="{ perm: 'docker:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="emit('create')"
        >{{ t('docker.createVolume') }}</FpButton
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
      virtual
      virtual-scroll-height="480px"
    >
      <FpColumn field="name" :header="t('docker.name')" style="min-width: 160px" />
      <FpColumn field="driver" :header="t('docker.driver')" style="width: 100px" />
      <FpColumn field="mountpoint" :header="t('docker.mountpoint')" style="min-width: 220px" />
      <FpColumn :header="t('docker.actions')" style="width: 130px" frozen align-frozen="right">
        <template #body="{ data }">
          <FpButton
            v-permission="{ perm: 'docker:delete', mode: 'view' }"
            variant="danger"
            icon="oi oi-trash"
            @click="emit('remove', data)"
            >{{ t('docker.remove') }}</FpButton
          >
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { DockerVolume } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * Docker 卷列表（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 三列（名称/驱动/挂载点）+ 一个删除动作，是五个页签里最简的一屏——但它的
 * 存在不是冗余：卷是运维最容易「看着没用其实在用」的资源，`mountpoint` 列
 * 让「这个卷挂到哪去了」不必开终端就能回答。
 */
withDefaults(
  defineProps<{
    rows: DockerVolume[]
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  create: []
  remove: [row: DockerVolume]
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
</style>
