<template>
  <div class="host-overview">
    <section class="panel">
      <header class="panel-header">
        <span class="panel-title">{{ t('toolbox.system.title') }}</span>
      </header>
      <dl class="kv-grid">
        <div class="cell">
          <dt>{{ t('toolbox.system.hostname') }}</dt>
          <dd class="mono">{{ host.system.hostname || '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.system.os') }}</dt>
          <dd>{{ host.system.os_name || host.system.os_id || '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.system.kernel') }}</dt>
          <dd class="mono">{{ host.system.kernel_release || '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.system.arch') }}</dt>
          <dd class="mono">{{ host.system.arch || '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.system.uptime') }}</dt>
          <dd class="mono">{{ formatUptime(host.system.uptime_secs) }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.system.bootTime') }}</dt>
          <dd class="mono">{{ bootTime }}</dd>
        </div>
      </dl>
    </section>

    <section class="panel">
      <header class="panel-header">
        <span class="panel-title">{{ t('toolbox.hardware.title') }}</span>
      </header>
      <dl class="kv-grid">
        <div class="cell cell-wide">
          <dt>{{ t('toolbox.hardware.cpuModel') }}</dt>
          <dd :title="host.cpu.model">{{ host.cpu.model || '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.hardware.cores') }}</dt>
          <dd class="mono">
            {{
              t('toolbox.hardware.coresValue', {
                logical: host.cpu.logical_cores,
                physical: host.cpu.physical_cores ?? '—',
              })
            }}
          </dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.hardware.load') }}</dt>
          <dd class="mono">{{ formatLoad(host.cpu.load_average) }}</dd>
        </div>
        <div class="cell cell-wide">
          <dt>{{ t('toolbox.hardware.memory') }}</dt>
          <dd class="usage">
            <FpProgress
              :value="memPercent"
              :class="['usage-bar', `tone-${usageTone(memPercent)}`]"
            />
            <span class="mono usage-text">
              {{
                t('toolbox.hardware.usage', {
                  used: formatSize(host.memory.total_bytes - host.memory.available_bytes),
                  total: formatSize(host.memory.total_bytes),
                  percent: memPercent,
                })
              }}
            </span>
          </dd>
        </div>
        <div class="cell cell-wide">
          <dt>{{ t('toolbox.hardware.swap') }}</dt>
          <dd v-if="host.memory.swap_total_bytes > 0" class="usage">
            <FpProgress
              :value="swapPercent"
              :class="['usage-bar', `tone-${usageTone(swapPercent)}`]"
            />
            <span class="mono usage-text">
              {{
                t('toolbox.hardware.usage', {
                  used: formatSize(host.memory.swap_used_bytes),
                  total: formatSize(host.memory.swap_total_bytes),
                  percent: swapPercent,
                })
              }}
            </span>
          </dd>
          <dd v-else class="text-muted">{{ t('toolbox.hardware.noSwap') }}</dd>
        </div>
      </dl>
    </section>

    <section class="panel">
      <header class="panel-header">
        <span class="panel-title">{{ t('toolbox.time.title') }}</span>
        <FpTag
          :severity="skewed ? 'warning' : 'success'"
          :value="skewed ? t('toolbox.time.skewed') : t('toolbox.time.inSync')"
          dot
        />
      </header>
      <dl class="kv-grid">
        <div class="cell">
          <dt>{{ t('toolbox.time.timezone') }}</dt>
          <dd class="mono">{{ host.time.timezone ?? t('toolbox.unknown') }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.time.offset') }}</dt>
          <dd class="mono">{{ host.time.utc_offset ? `UTC${host.time.utc_offset}` : '—' }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.time.serverNow') }}</dt>
          <dd class="mono">{{ serverNow }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.time.skew') }}</dt>
          <dd class="mono" :class="{ 'text-warning': skewed }">
            {{ t('toolbox.time.skewValue', { secs: skewSecs > 0 ? `+${skewSecs}` : skewSecs }) }}
          </dd>
        </div>
      </dl>
      <FpInlineMessage v-if="skewed" severity="warn" class="skew-note">
        {{ t('toolbox.time.skewHint', { limit: CLOCK_SKEW_WARN_SECS }) }}
      </FpInlineMessage>
    </section>

    <section class="panel">
      <header class="panel-header">
        <span class="panel-title">{{ t('toolbox.panel.title') }}</span>
        <FpTag
          v-if="host.panel.running_as_root"
          severity="warning"
          :value="t('toolbox.panel.root')"
        />
      </header>
      <dl class="kv-grid">
        <div class="cell">
          <dt>{{ t('toolbox.panel.version') }}</dt>
          <dd class="mono">v{{ host.panel.version }}</dd>
        </div>
        <div class="cell">
          <dt>PID</dt>
          <dd class="mono">{{ host.panel.pid }}</dd>
        </div>
        <div class="cell">
          <dt>UID</dt>
          <dd class="mono">{{ host.panel.uid }}</dd>
        </div>
        <div class="cell">
          <dt>{{ t('toolbox.panel.generatedAt') }}</dt>
          <dd class="mono">{{ generatedAt }}</dd>
        </div>
      </dl>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpTag from '@/components/ui/FpTag.vue'
import FpProgress from '@/components/ui/FpProgress.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import type { HostDetails } from '@/api/toolbox'
import { formatSize } from '@/utils/format'
import { formatUptime } from '@/utils/inspection'
import {
  CLOCK_SKEW_WARN_SECS,
  formatLoad,
  isClockSkewed,
  usageTone,
  usedPercent,
} from '@/utils/hostInfo'

const props = defineProps<{ host: HostDetails; skewSecs: number }>()

const { t } = useI18n()

const memPercent = computed(() =>
  usedPercent(props.host.memory.total_bytes, props.host.memory.available_bytes),
)
const swapPercent = computed(() =>
  usedPercent(
    props.host.memory.swap_total_bytes,
    props.host.memory.swap_total_bytes - props.host.memory.swap_used_bytes,
  ),
)
const skewed = computed(() => isClockSkewed(props.skewSecs))
const bootTime = computed(() =>
  props.host.system.boot_time ? new Date(props.host.system.boot_time).toLocaleString() : '—',
)
const serverNow = computed(() => new Date(props.host.time.now).toLocaleString())
const generatedAt = computed(() => new Date(props.host.generated_at).toLocaleTimeString())
</script>

<style scoped>
.host-overview {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fp-space-4);
}
.kv-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
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
.usage {
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
.text-warning {
  color: var(--fp-warning);
}
.skew-note {
  margin-top: var(--fp-space-3);
}
@media (max-width: 960px) {
  .host-overview {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
