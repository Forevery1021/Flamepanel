<template>
  <section class="panel">
    <header class="panel-header">
      <span class="panel-title">{{ t('toolbox.disks.title') }}</span>
    </header>
    <ul v-if="disks.length" class="disk-list">
      <li v-for="d in disks" :key="d.mount_point" class="disk-item">
        <div class="disk-head">
          <span class="mono disk-mount">{{ d.mount_point }}</span>
          <span class="text-muted mono disk-meta">{{ d.device }} · {{ d.fs_type }}</span>
        </div>
        <div class="disk-usage">
          <FpProgress
            :value="diskUsedPercent(d)"
            :class="['usage-bar', `tone-${usageTone(diskUsedPercent(d))}`]"
          />
          <span class="mono usage-text">
            {{
              t('toolbox.hardware.usage', {
                used: formatSize(d.total_bytes - d.available_bytes),
                total: formatSize(d.total_bytes),
                percent: diskUsedPercent(d),
              })
            }}
          </span>
        </div>
      </li>
    </ul>
    <p v-else class="text-muted">{{ t('toolbox.disks.empty') }}</p>
  </section>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpProgress from '@/components/ui/FpProgress.vue'
import type { HostDisk } from '@/api/toolbox'
import { formatSize } from '@/utils/format'
import { diskUsedPercent, usageTone } from '@/utils/hostInfo'

defineProps<{ disks: HostDisk[] }>()

const { t } = useI18n()
</script>

<style scoped>
.disk-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.disk-head {
  display: flex;
  align-items: baseline;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-1);
  min-width: 0;
}
.disk-mount {
  font-weight: 600;
}
.disk-meta {
  font-size: var(--fp-font-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.disk-usage {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
}
.usage-bar {
  flex: 1;
  height: 6px;
}
.usage-text {
  flex-shrink: 0;
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.tone-success :deep(.p-progressbar-value) {
  background: var(--fp-success);
}
.tone-warning :deep(.p-progressbar-value) {
  background: var(--fp-warning);
}
.tone-danger :deep(.p-progressbar-value) {
  background: var(--fp-danger);
}
</style>
