<template>
  <LayoutContent
    :title="t('toolbox.title')"
    :description="t('pageDesc.toolbox')"
    :summary="summary"
    reload
    @reload="refresh"
  >
    <section class="tools-section">
      <h3 class="section-title">{{ t('toolbox.tools.title') }}</h3>
      <ToolboxToolGrid />
    </section>

    <FpStatePanel
      :loading="loading && !host"
      :error="host ? null : error"
      :empty="!loading && !error && !host"
      retryable
      :title="error ? t('toolbox.loadFailed') : ''"
      :empty-title="t('toolbox.empty')"
      @retry="refresh"
    >
      <div v-if="host" class="host-sections">
        <HostOverviewCard :host="host" :skew-secs="skewSecs" />
        <HostNetworkCard
          :interfaces="host.interfaces"
          :dns="host.dns"
          :hosts="host.hosts"
          :hosts-truncated="host.hosts_truncated"
        />
        <HostDiskCard :disks="host.disks" />
      </div>
    </FpStatePanel>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutContent, { type LayoutSummaryItem } from '@/components/ui/LayoutContent.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import ToolboxToolGrid from '@/components/toolbox/ToolboxToolGrid.vue'
import HostOverviewCard from '@/components/toolbox/HostOverviewCard.vue'
import HostNetworkCard from '@/components/toolbox/HostNetworkCard.vue'
import HostDiskCard from '@/components/toolbox/HostDiskCard.vue'
import { useHostDetails } from '@/composables/useHostDetails'
import { formatUptime } from '@/utils/inspection'

/**
 * 系统工具箱：常用运维工具入口（按权限过滤）+ 主机详情只读汇总。
 * 主机详情数据来自 `GET /api/toolbox/host`（`toolbox:read`）。
 */
const { t } = useI18n()
const { host, skewSecs, loading, error, refresh } = useHostDetails()

const summary = computed<LayoutSummaryItem[]>(() => {
  const h = host.value
  if (!h) return []
  return [
    { label: t('toolbox.system.hostname'), value: h.system.hostname || '—', tone: 'brand' },
    { label: t('toolbox.system.os'), value: h.system.os_name || h.system.os_id || '—' },
    { label: t('toolbox.system.uptime'), value: formatUptime(h.system.uptime_secs) },
    {
      label: t('toolbox.hardware.cores'),
      value: String(h.cpu.logical_cores),
      tone: 'info',
    },
  ]
})
</script>

<style scoped>
.tools-section {
  margin-bottom: var(--fp-space-5);
}
.section-title {
  margin: 0 0 var(--fp-space-3);
  font-size: var(--fp-font-md);
  font-weight: 600;
  color: var(--fp-text-primary);
}
.host-sections {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
</style>
