<template>
  <FpDrawer
    v-model="visible"
    :header="
      t('certificate.renewalHistoryTitle', {
        domain: certificate ? formatDomains(certificate.domains) : '',
      })
    "
  >
    <div class="renewal-body">
      <FpStatePanel :loading="loading" :error="error" retryable @retry="emit('refresh')">
        <template v-if="history">
          <dl class="renewal-summary">
            <div>
              <dt>{{ t('certificate.autoRenew') }}</dt>
              <dd>
                <FpTag :severity="autoRenewState.severity" :value="autoRenewState.label" />
              </dd>
            </div>
            <div>
              <dt>{{ t('certificate.expiresAt') }}</dt>
              <dd>{{ formatDateTime(history.expires_at, t('certificate.neverRenewed')) }}</dd>
            </div>
            <div>
              <dt>{{ t('certificate.renewalWindow') }}</dt>
              <dd>
                {{
                  history.renewal_window_starts_at
                    ? t('certificate.renewalWindowFrom', {
                        time: formatDateTime(history.renewal_window_starts_at, ''),
                      })
                    : t('certificate.neverRenewed')
                }}
              </dd>
            </div>
            <div>
              <dt>{{ t('certificate.lastRenewedAt') }}</dt>
              <dd>{{ formatDateTime(history.last_renewed_at, t('certificate.neverRenewed')) }}</dd>
            </div>
            <div>
              <dt>{{ t('certificate.scanInterval') }}</dt>
              <dd>{{ t('certificate.scanIntervalValue', { n: history.scan_interval_hours }) }}</dd>
            </div>
          </dl>

          <div class="renewal-body__actions">
            <FpButton
              variant="ghost"
              size="small"
              icon="oi oi-refresh"
              :loading="fetching"
              @click="emit('refresh')"
            >
              {{ t('common.retry') }}
            </FpButton>
          </div>

          <p v-if="!history.entries.length" class="renewal-empty">
            {{ t('certificate.timelineEmpty') }}
          </p>
          <ol v-else class="renewal-timeline">
            <li
              v-for="entry in history.entries"
              :key="entry.event_id"
              class="renewal-timeline__item"
              :class="`renewal-timeline__item--${KIND_SEVERITY[entry.kind]}`"
            >
              <div class="renewal-timeline__head">
                <FpTag
                  :severity="KIND_SEVERITY[entry.kind]"
                  :value="t(`certificate.event_${entry.kind}`)"
                />
                <time class="renewal-timeline__time">{{ formatDateTime(entry.at, '') }}</time>
              </div>
              <p
                v-if="entry.error"
                class="renewal-timeline__detail renewal-timeline__detail--error"
              >
                {{ entry.error }}
              </p>
              <p v-else-if="entry.expires_at" class="renewal-timeline__detail">
                {{
                  t('certificate.eventExpiresAt', { time: formatDateTime(entry.expires_at, '') })
                }}
              </p>
              <p v-else-if="entry.source" class="renewal-timeline__detail">
                {{
                  t('certificate.eventSource', {
                    source: te(`certificate.source_${entry.source}`)
                      ? t(`certificate.source_${entry.source}`)
                      : entry.source,
                  })
                }}
              </p>
            </li>
          </ol>
          <p v-if="history.truncated" class="renewal-empty">
            {{ t('certificate.timelineTruncated', { n: history.entries.length }) }}
          </p>
        </template>
      </FpStatePanel>
    </div>
  </FpDrawer>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Certificate, CertificateRenewalHistory } from '@/api/certificates'
import type { RenewalEventKind } from '@/api/generated'
import { formatDateTime, formatDomains } from '@/utils/websiteFormat'
import FpDrawer from '@/components/ui/FpDrawer.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'

/**
 * 证书续签记录抽屉（S-10）。
 *
 * 上半部是续签概况（开关态 / 到期 / 续签窗口起点 / 最近签发 / 扫描周期），下半部是
 * 时间线（签发、自动续签成功与失败、到期预警，最新在前）。数据来自事件落库，
 * 后端已合并「自动续签内部的那次签发」，这里只负责展示。
 *
 * 与 `ProjectLogsDrawer` 同构：请求状态全由父级（`useWebsiteCertificates`）透传，
 * 本组件不发请求；「刷新」放在正文上方，成功态也可重拉。
 */
const props = withDefaults(
  defineProps<{
    certificate?: Certificate | null
    history?: CertificateRenewalHistory | null
    loading?: boolean
    fetching?: boolean
    error?: string | null
  }>(),
  { certificate: null, history: null, loading: false, fetching: false, error: null },
)

const visible = defineModel<boolean>({ required: true })

const emit = defineEmits<{ refresh: [] }>()

const { t, te } = useI18n()

type Severity = 'success' | 'danger' | 'warning' | 'neutral'

const KIND_SEVERITY: Record<RenewalEventKind, Severity> = {
  issued: 'neutral',
  auto_renewed: 'success',
  auto_renew_failed: 'danger',
  expiry_warning: 'warning',
}

const autoRenewState = computed<{ label: string; severity: Severity }>(() => {
  const h = props.history
  if (!h?.supports_auto_renew) return { label: t('certificate.autoRenewNa'), severity: 'neutral' }
  return h.auto_renew
    ? { label: t('certificate.autoRenewOn'), severity: 'success' }
    : { label: t('certificate.autoRenewOff'), severity: 'warning' }
})
</script>

<style scoped>
.renewal-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  min-height: 200px;
}
.renewal-summary {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: var(--fp-space-3);
  margin: 0;
}
.renewal-summary dt {
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.renewal-summary dd {
  margin: var(--fp-space-1) 0 0;
}
.renewal-body__actions {
  display: flex;
  justify-content: flex-end;
}
.renewal-empty {
  margin: 0;
  color: var(--fp-text-secondary);
  font-size: 13px;
}
.renewal-timeline {
  list-style: none;
  margin: 0;
  padding: 0 0 0 var(--fp-space-3);
  border-left: 2px solid var(--fp-border);
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.renewal-timeline__item {
  position: relative;
}
.renewal-timeline__item::before {
  content: '';
  position: absolute;
  left: calc(-1 * var(--fp-space-3) - 6px);
  top: 6px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--fp-border);
}
.renewal-timeline__item--success::before {
  background: var(--fp-success);
}
.renewal-timeline__item--danger::before {
  background: var(--fp-danger);
}
.renewal-timeline__item--warning::before {
  background: var(--fp-warning);
}
.renewal-timeline__head {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.renewal-timeline__time {
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.renewal-timeline__detail {
  margin: var(--fp-space-1) 0 0;
  font-size: 13px;
  word-break: break-all;
}
.renewal-timeline__detail--error {
  color: var(--fp-danger);
}
</style>
