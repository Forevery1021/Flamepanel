<template>
  <div class="panel">
    <FpTable
      :rows="apps"
      :loading="loading"
      :empty-text="t('appStore.empty')"
      virtual
      virtual-scroll-height="560px"
    >
      <FpColumn field="name" :header="t('appStore.colName')" style="min-width: 140px" />
      <FpColumn field="package_key" :header="t('appStore.colPackage')" style="min-width: 120px" />
      <FpColumn field="version" :header="t('appStore.colVersion')" style="width: 100px" />
      <FpColumn :header="t('appStore.colMode')" style="width: 100px">
        <template #body="{ data }">
          <FpTag :severity="modeTagType(data.mode)" :value="modeLabel(data.mode, t)" />
        </template>
      </FpColumn>
      <FpColumn :header="t('appStore.colStatus')" style="width: 100px">
        <template #body="{ data }">
          <FpTag
            :severity="data.status === 'running' ? 'success' : 'danger'"
            :value="data.status"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('appStore.colAccess')" style="min-width: 160px">
        <template #body="{ data }">
          <a v-if="data.access_url" class="access-link" :href="data.access_url" target="_blank">{{
            data.access_url
          }}</a>
          <span v-else>-</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('appStore.colActions')" style="width: 240px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <template v-if="isNativeMode(data)">
              <FpButton
                v-permission="{ perm: 'app_store:update', mode: 'view' }"
                variant="ghost"
                :disabled="data.status === 'running'"
                @click="emit('nativeStart', data)"
              >{{ t('appStore.start') }}</FpButton>
              <FpButton
                v-permission="{ perm: 'app_store:update', mode: 'view' }"
                variant="ghost"
                :disabled="data.status !== 'running'"
                @click="emit('nativeStop', data)"
              >{{ t('appStore.stop') }}</FpButton>
              <FpButton
                v-permission="{ perm: 'app_store:update', mode: 'view' }"
                variant="ghost"
                @click="emit('nativeRestart', data)"
              >{{ t('appStore.restart') }}</FpButton>
            </template>
            <FpButton variant="ghost" @click="emit('logs', data)">{{
              t('appStore.logs')
            }}</FpButton>
            <FpButton
              v-permission="{ perm: 'app_store:update', mode: 'view' }"
              variant="ghost"
              @click="emit('upgrade', data)"
            >{{ t('appStore.upgrade') }}</FpButton>
            <FpButton
              v-permission="{ perm: 'app_store:delete', mode: 'view' }"
              variant="danger"
              @click="emit('uninstall', data)"
            >
              {{ t('appStore.uninstall') }}
            </FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { isNativeMode, modeLabel, modeTagType } from '@/utils/appStoreFormat'
import type { InstalledApp } from '@/api/appStore'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 已安装应用表（Doc/23 §3.3 T2-4 视图拆分）。
 *
 * 展示与操作解耦：行内六个操作（原生启停/重启、日志、升级、卸载）全部上抛，
 * 具体编排在 `useAppStoreCatalog`——表格自身不持有任何请求状态。
 *
 * 列宽与冻结列沿用原实现（`frozen` 操作列在窄屏下保持可点）。
 */
withDefaults(
  defineProps<{
    apps: InstalledApp[]
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  uninstall: [app: InstalledApp]
  upgrade: [app: InstalledApp]
  logs: [app: InstalledApp]
  nativeStart: [app: InstalledApp]
  nativeStop: [app: InstalledApp]
  nativeRestart: [app: InstalledApp]
}>()

const { t } = useI18n()
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.access-link {
  color: var(--fp-brand);
  text-decoration: none;
}
.access-link:hover {
  text-decoration: underline;
}
</style>
