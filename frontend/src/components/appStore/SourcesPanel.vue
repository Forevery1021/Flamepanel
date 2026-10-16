<template>
  <div class="panel">
    <div class="sources-toolbar">
      <FpButton
        v-permission="{ perm: 'app_store:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="emit('create')"
      >{{ t('appStore.addSource') }}</FpButton>
    </div>
    <FpTable
      :rows="sources"
      :loading="loading"
      :empty-text="t('appStore.emptySources')"
    >
      <FpColumn field="name" :header="t('appStore.colSourceName')" style="min-width: 140px" />
      <FpColumn :header="t('appStore.colSourceType')" style="width: 90px">
        <template #body="{ data }">
          <FpTag severity="info" :value="data.source_type.toUpperCase()" />
        </template>
      </FpColumn>
      <FpColumn field="url" :header="t('appStore.colSourceUrl')" style="min-width: 220px" />
      <FpColumn :header="t('appStore.colSourceKey')" style="width: 100px">
        <template #body="{ data }">
          <FpTag
            :severity="data.public_key ? 'success' : 'warning'"
            :value="data.public_key ? t('appStore.keyConfigured') : t('appStore.keyMissing')"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('appStore.colSyncedAt')" style="width: 160px">
        <template #body="{ data }">
          <span>{{
            data.last_synced_at ? new Date(data.last_synced_at).toLocaleString() : '-'
          }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('appStore.colActions')" style="width: 320px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'app_store:update', mode: 'view' }"
              variant="primary"
              :loading="syncingSourceId === data.id"
              @click="emit('sync', data)"
            >{{ t('appStore.syncNow') }}</FpButton>
            <FpButton
              v-permission="{ perm: 'app_store:update', mode: 'view' }"
              variant="ghost"
              @click="emit('editKey', data)"
            >{{ t('appStore.editPublicKey') }}</FpButton>
            <FpButton
              v-permission="{ perm: 'app_store:update', mode: 'view' }"
              variant="ghost"
              @click="emit('toggle', data)"
            >{{ data.enabled ? t('appStore.disableSource') : t('appStore.enableSource') }}</FpButton>
            <FpButton
              v-permission="{ perm: 'app_store:delete', mode: 'view' }"
              variant="danger"
              @click="emit('remove', data)"
            >{{ t('appStore.deleteSource') }}</FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { AppSource } from '@/api/appStore'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 远程应用源列表（Doc/23 §3.3 T2-4 视图拆分；M-A Issue 1.4）。
 *
 * 展示与编排解耦：同步 / 公钥编辑 / 启停 / 删除四类操作全部上抛，
 * 确认弹窗与请求编排在 `useAppStoreSources`（两处二次确认属安全语义，
 * 不在展示层复制）。
 */
defineProps<{
  sources: AppSource[]
  loading?: boolean
  syncingSourceId?: number | null
}>()

const emit = defineEmits<{
  sync: [source: AppSource]
  editKey: [source: AppSource]
  toggle: [source: AppSource]
  remove: [source: AppSource]
  create: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.sources-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--fp-space-3);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
</style>
