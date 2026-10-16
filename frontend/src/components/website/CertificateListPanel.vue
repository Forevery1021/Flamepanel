<template>
  <div class="panel">
    <FpStatePanel
      :loading="loading"
      :error="error"
      :empty="!certificates.length && !loading && !error"
      retryable
      :empty-title="t('common.noData')"
      :empty-desc="t('pageDesc.emptyHint')"
      @retry="emit('retry')"
    >
      <template v-if="$slots.emptyAction" #emptyAction><slot name="emptyAction" /></template>
      <FpTable :rows="certificates" :empty-text="t('common.noData')" striped-rows>
        <FpColumn field="id" :header="t('website.id')" style="width: 60px" />
        <FpColumn :header="t('certificate.domains')">
          <template #body="{ data }">
            {{ formatDomains(data.domains) }}
          </template>
        </FpColumn>
        <FpColumn :header="t('certificate.source')" style="width: 110px">
          <template #body="{ data }">
            <FpTag :severity="data.source === 'self_signed' ? 'neutral' : 'success'">
              {{ t('certificate.source_' + data.source) }}
            </FpTag>
          </template>
        </FpColumn>
        <FpColumn :header="t('certificate.expiresAt')" style="width: 190px">
          <template #body="{ data }">
            <FpTag
              v-if="certState(data) === 'expired'"
              severity="danger"
              :value="t('certificate.expired')"
            />
            <FpTag
              v-else-if="certState(data) === 'expiring'"
              severity="warning"
              :value="t('certificate.expiringSoon')"
            />
            <span v-else>{{ formatDateTime(data.expires_at, t('certificate.neverRenewed')) }}</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('certificate.autoRenew')" style="width: 90px">
          <template #body="{ data }">
            <span
              v-if="!supportsAutoRenew(data)"
              v-tooltip="t('certificate.autoRenewUnsupported')"
              class="auto-renew-na"
            >
              <FpSwitch :model-value="false" disabled />
            </span>
            <FpSwitch
              v-else
              v-permission="{ perm: 'certificate:update', mode: 'view' }"
              :model-value="data.auto_renew"
              @update:model-value="(v: boolean) => emit('toggleAutoRenew', data, v)"
            />
          </template>
        </FpColumn>
        <FpColumn :header="t('certificate.boundWebsite')" style="width: 130px">
          <template #body="{ data }">
            {{ data.website_id ?? t('certificate.notBound') }}
          </template>
        </FpColumn>
        <FpColumn :header="t('common.operation')" style="width: 300px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton
                v-permission="{ perm: 'certificate:update', mode: 'view' }"
                variant="link"
                @click="emit('bind', data)"
                >{{ t('certificate.bind') }}</FpButton
              >
              <FpButton
                v-if="data.website_id"
                v-permission="{ perm: 'certificate:update', mode: 'view' }"
                variant="link"
                @click="emit('unbind', data)"
                >{{ t('certificate.unbind') }}</FpButton
              >
              <FpButton variant="link" @click="emit('history', data)">{{
                t('certificate.renewalHistory')
              }}</FpButton>
              <FpButton
                v-if="supportsAutoRenew(data)"
                v-permission="{ perm: 'certificate:update', mode: 'view' }"
                variant="link"
                :loading="renewingId === data.id"
                @click="emit('renew', data)"
                >{{ t('certificate.renewNow') }}</FpButton
              >
              <FpButton
                v-permission="{ perm: 'certificate:delete', mode: 'view' }"
                variant="link"
                @click="emit('delete', data)"
                >{{ t('common.delete') }}</FpButton
              >
            </div>
          </template>
        </FpColumn>
      </FpTable>
    </FpStatePanel>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { supportsAutoRenew, type Certificate } from '@/api/certificates'
import { certState, formatDateTime, formatDomains } from '@/utils/websiteFormat'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'

/**
 * 证书列表（Doc/23 §3.3 T2-4 视图拆分第二站；主线三 §6.1）。
 *
 * 展示与编排解耦：绑定 / 解绑 / 续签 / 删除四类操作全部上抛，
 * `auto_renew` 开关经 `emit('toggleAutoRenew', row, enabled)` 上抛（不改 props，
 * 行数据回写在数据层，见 `useWebsiteCertificates.toggleAutoRenew`）。
 *
 * 到期列的三种形态（已过期 / 30 天内到期 / 正常时间）由纯函数 `certState` 判定，
 * 与后端 `needs_renewal` 的 30 天续签窗口同源。
 *
 * S-10：自签 / 上传证书没有签发方可续，自动续签开关渲染为禁用态并给出原因提示
 * （后端对其开启返回 400）；「续签记录」上抛 `history`，由视图层打开时间线抽屉。
 */
defineProps<{
  certificates: Certificate[]
  loading?: boolean
  error?: string | null
  renewingId?: number | null
}>()

const emit = defineEmits<{
  retry: []
  bind: [row: Certificate]
  unbind: [row: Certificate]
  renew: [row: Certificate]
  delete: [row: Certificate]
  toggleAutoRenew: [row: Certificate, enabled: boolean]
  history: [row: Certificate]
}>()

const { t } = useI18n()
</script>

<style scoped>
.auto-renew-na {
  display: inline-flex;
  cursor: not-allowed;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
