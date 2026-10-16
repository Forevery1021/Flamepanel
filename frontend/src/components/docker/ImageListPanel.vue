<template>
  <div class="panel">
    <div class="toolbar">
      <div class="search-wrap">
        <FpInput
          v-model="pullName"
          :placeholder="t('docker.imageNamePlaceholder')"
          @keyup.enter="emit('pull')"
        />
      </div>
      <FpButton
        v-permission="{ perm: 'docker:start', mode: 'view' }"
        variant="primary"
        icon="oi oi-download"
        :loading="pulling"
        @click="emit('pull')"
        >{{ t('docker.pull') }}</FpButton
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
      <FpColumn field="id" :header="t('docker.containerId')" style="width: 200px" />
      <FpColumn :header="t('docker.tags')" style="min-width: 240px">
        <template #body="{ data }">
          <FpTag v-for="tag in data.tags" :key="tag" severity="neutral" :value="tag" class="mr-1" />
          <span v-if="!data.tags || !data.tags.length" class="text-muted">{{
            data.repo_tags?.join(', ') || data.id
          }}</span>
        </template>
      </FpColumn>
      <FpColumn field="size" :header="t('docker.imageSize')" style="width: 100px">
        <template #body="{ data }">{{ formatSize(data.size) }}</template>
      </FpColumn>
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
import { formatSize } from '@/utils/format'
import type { DockerImage } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * Docker 镜像列表（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 拉取输入框经 `v-model:pull-name` 由父级持有（拉取成功后要清空，清空动作属数据层）；
 * 标签列有**两级回落**：`tags` 为空时读 Docker API 原生形态 `repo_tags`，
 * 再为空则回落镜像 ID——短 ID 也比空白单元格有用，这条回落链有用例锁死。
 */
withDefaults(
  defineProps<{
    rows: DockerImage[]
    loading?: boolean
    pulling?: boolean
  }>(),
  { loading: false, pulling: false },
)

const pullName = defineModel<string>('pullName', { default: '' })

const emit = defineEmits<{
  pull: []
  remove: [row: DockerImage]
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
.search-wrap {
  flex: 1;
  max-width: 320px;
  min-width: 220px;
}
.mr-1 {
  margin-right: 4px;
}
</style>
