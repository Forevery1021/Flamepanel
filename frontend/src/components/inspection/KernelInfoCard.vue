<template>
  <div class="panel kernel-card" data-testid="inspection-kernel">
    <div class="panel-header">
      <span class="panel-title">{{ t('inspection.kernel.title') }}</span>
      <div class="kernel-badges">
        <FpTag
          v-if="kernel.reboot_required"
          severity="warning"
          :value="t('inspection.kernel.rebootRequired')"
          dot
        />
        <FpTag v-if="kernel.lts === true" severity="success" :value="t('inspection.kernel.lts')" />
      </div>
    </div>

    <dl class="kernel-grid">
      <div class="cell cell-wide">
        <dt>{{ t('inspection.kernel.release') }}</dt>
        <dd class="mono">{{ kernel.release || '—' }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.arch') }}</dt>
        <dd class="mono">{{ kernel.arch || '—' }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.series') }}</dt>
        <dd class="mono">{{ kernel.series ?? '—' }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.seriesEol') }}</dt>
        <dd class="mono">{{ kernel.series_eol ?? '—' }}</dd>
      </div>
      <div class="cell cell-wide">
        <dt>{{ t('inspection.kernel.os') }}</dt>
        <dd>{{ kernel.os_name || kernel.os_id || '—' }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.hostname') }}</dt>
        <dd class="mono">{{ kernel.hostname || '—' }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.uptime') }}</dt>
        <dd class="mono">{{ formatUptime(kernel.uptime_secs) }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.virtualization') }}</dt>
        <dd class="mono">{{ virtualization }}</dd>
      </div>
      <div class="cell">
        <dt>{{ t('inspection.kernel.newestInstalled') }}</dt>
        <dd class="mono">{{ kernel.newest_installed_kernel ?? '—' }}</dd>
      </div>
      <div class="cell cell-full">
        <dt>{{ t('inspection.kernel.version') }}</dt>
        <dd class="mono version-line" :title="kernel.version">{{ kernel.version || '—' }}</dd>
      </div>
    </dl>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpTag from '@/components/ui/FpTag.vue'
import type { KernelInfo } from '@/api/inspection'
import { formatUptime } from '@/utils/inspection'

const props = defineProps<{ kernel: KernelInfo }>()

const { t } = useI18n()

const virtualization = computed(() => {
  const v = props.kernel.virtualization
  if (v === null || v === undefined) return t('inspection.kernel.unknown')
  if (v === 'none' || v === '') return t('inspection.kernel.bare')
  return v
})
</script>

<style scoped>
.kernel-badges {
  display: flex;
  gap: var(--fp-space-2);
}
.kernel-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--fp-space-3) var(--fp-space-4);
  margin: 0;
}
.cell {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.cell-wide {
  grid-column: span 2;
}
.cell-full {
  grid-column: 1 / -1;
}
.cell dt {
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
}
.cell dd {
  margin: 0;
  font-size: 13px;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.version-line {
  color: var(--fp-text-secondary);
}
@media (max-width: 720px) {
  .kernel-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
