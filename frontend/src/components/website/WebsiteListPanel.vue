<template>
  <div class="panel">
    <FpStatePanel
      :loading="loading"
      :error="error"
      :empty="!total && !loading && !error"
      retryable
      :empty-title="t('common.noData')"
      :empty-desc="$slots.emptyAction ? t('pageDesc.emptyHint') : ''"
      @retry="emit('retry')"
    >
      <template v-if="$slots.emptyAction" #emptyAction>
        <slot name="emptyAction" />
      </template>
      <FpTable
        :rows="rows"
        :loading="loading"
        :empty-text="t('common.noData')"
        :first="first"
        striped-rows
      >
        <FpColumn field="id" :header="t('website.id')" style="width: 60px" />
        <FpColumn field="name" :header="t('website.name')" />
        <FpColumn field="domain" :header="t('website.domain')" />
        <FpColumn field="root_path" :header="t('website.rootPath')" />
        <FpColumn :header="t('website.siteType')" style="width: 110px">
          <template #body="{ data }">
            <FpTag :severity="data.site_type === 'static' ? 'info' : 'success'">
              {{ t('website.siteType_' + (data.site_type || 'static')) }}
            </FpTag>
          </template>
        </FpColumn>
        <FpColumn :header="t('website.rewritePreset')" style="width: 120px">
          <template #body="{ data }">
            <FpTag
              :severity="data.rewrite_preset ? 'success' : 'info'"
              :value="data.rewrite_preset || t('website.rewriteNone')"
            />
          </template>
        </FpColumn>
        <FpColumn field="node_id" :header="t('website.nodeId')" style="width: 80px" />
        <FpColumn :header="t('website.status')" style="width: 100px">
          <template #body="{ data }">
            <FpTag :severity="data.status === 'active' ? 'success' : 'info'">
              {{ data.status === 'active' ? t('website.active') : t('website.inactive') }}
            </FpTag>
          </template>
        </FpColumn>
        <FpColumn field="created_at" :header="t('website.createdAt')" style="width: 180px" />
        <FpColumn :header="t('common.operation')" style="width: 260px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton
                v-permission="{ perm: 'website:update', mode: 'view' }"
                variant="link"
                @click="emit('edit', data)"
                >{{ t('common.edit') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'website:update', mode: 'view' }"
                variant="link"
                @click="emit('switchEngine', data)"
                >{{ t('website.switchEngine') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'website:update', mode: 'view' }"
                variant="link"
                :loading="deployingId === data.id"
                :title="t('website.deployHint')"
                @click="emit('deploy', data)"
                >{{ t('website.deploy') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'website:update', mode: 'view' }"
                variant="link"
                @click="emit('bind', data)"
                >{{ t('website.bind') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'website:update', mode: 'view' }"
                variant="link"
                @click="emit('rewrite', data)"
                >{{ t('website.rewritePreset') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'website:delete', mode: 'view' }"
                variant="link"
                @click="emit('delete', data)"
                >{{ t('common.delete') }}</FpButton
              >
            </div>
          </template>
        </FpColumn>
      </FpTable>
      <FpPagination
        v-if="total > pageSize"
        :first="first"
        :rows="pageSize"
        :total="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(v: number) => emit('update:first', v)"
      />
    </FpStatePanel>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Website } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'

/**
 * 网站列表（Doc/23 §3.3 T2-4 视图拆分第二站）。
 *
 * **纯展示 + 事件上抛**：本组件不持有任何请求状态，五类行内操作（编辑 / 切换引擎 /
 * 绑定 / 伪静态 / 删除）以及状态面板的重试、分页翻页全部交由父级编排
 * （`useWebsiteList`）。列宽与冻结操作列沿用原实现（拆分不改视觉契约）。
 */
withDefaults(
  defineProps<{
    rows: Website[]
    loading?: boolean
    error?: string | null
    total?: number
    first?: number
    pageSize?: number
    /** 正在部署的站点 ID */
    deployingId?: number | null
  }>(),
  { loading: false, error: null, total: 0, first: 0, pageSize: 20, deployingId: null },
)

const emit = defineEmits<{
  edit: [row: Website]
  switchEngine: [row: Website]
  deploy: [row: Website]
  bind: [row: Website]
  rewrite: [row: Website]
  delete: [row: Website]
  retry: []
  'update:first': [value: number]
}>()

const { t } = useI18n()
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
