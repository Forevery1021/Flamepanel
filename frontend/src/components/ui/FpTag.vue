<template>
  <Tag
    :value="value ?? ''"
    :severity="sev"
    :rounded="rounded"
    :class="['fp-tag', { 'fp-tag-dot': dot }]"
  >
    <template v-if="dot || $slots.default" #default>
      <span v-if="dot" class="fp-tag__dot" :class="`is-${sev}`" />
      <slot>{{ value }}</slot>
    </template>
  </Tag>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import Tag from 'openvue/tag'

type FpSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

const props = withDefaults(
  defineProps<{
    value?: string
    severity?: FpSeverity
    dot?: boolean
    rounded?: boolean
  }>(),
  { value: '', severity: 'neutral', dot: false, rounded: true },
)

const severityMap: Record<FpSeverity, string | undefined> = {
  success: 'success',
  warning: 'warn',
  danger: 'danger',
  info: 'info',
  neutral: 'secondary',
}

const sev = computed(() => severityMap[props.severity])
</script>

<style scoped>
.fp-tag {
  font-family: var(--fp-font-sans);
}
.fp-tag__dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 999px;
  margin-right: 6px;
  vertical-align: 2px;
}
.fp-tag__dot.is-success {
  background: var(--fp-success);
  box-shadow: 0 0 0 3px var(--fp-success-soft);
  animation: fp-pulse 2s var(--fp-ease-out) infinite;
}
.fp-tag__dot.is-warning {
  background: var(--fp-warning);
  box-shadow: 0 0 0 3px var(--fp-warning-soft);
}
.fp-tag__dot.is-danger {
  background: var(--fp-danger);
  box-shadow: 0 0 0 3px var(--fp-danger-soft);
}
.fp-tag__dot.is-info {
  background: var(--fp-info);
  box-shadow: 0 0 0 3px var(--fp-info-soft);
}
.fp-tag__dot.is-neutral {
  background: var(--fp-text-muted);
}
@keyframes fp-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}
@media (prefers-reduced-motion: reduce) {
  .fp-tag__dot.is-success {
    animation: none;
  }
}
</style>
