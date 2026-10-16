<template>
  <div class="panel score-card" :class="`is-${tone}`" data-testid="inspection-score">
    <div class="score-ring" role="img" :aria-label="`${t('inspection.score')} ${summary.score}`">
      <svg viewBox="0 0 120 120" aria-hidden="true">
        <circle class="ring-track" cx="60" cy="60" r="52" />
        <circle
          class="ring-value"
          cx="60"
          cy="60"
          r="52"
          :stroke-dasharray="`${circumference} ${circumference}`"
          :stroke-dashoffset="dashOffset"
        />
      </svg>
      <div class="score-number">
        <span class="score-value">{{ summary.score }}</span>
        <span class="score-grade">{{ t('inspection.grade', { grade: summary.grade }) }}</span>
      </div>
    </div>

    <div class="score-meta">
      <div class="score-title">{{ t('inspection.score') }}</div>
      <dl class="score-stats">
        <div class="stat">
          <dt>{{ t('inspection.summary.total') }}</dt>
          <dd>{{ summary.total }}</dd>
        </div>
        <div class="stat is-success">
          <dt>{{ t('inspection.summary.passed') }}</dt>
          <dd>{{ summary.passed }}</dd>
        </div>
        <div class="stat is-danger">
          <dt>{{ t('inspection.summary.failed') }}</dt>
          <dd>{{ summary.failed }}</dd>
        </div>
        <div class="stat is-muted">
          <dt>{{ t('inspection.summary.unknown') }}</dt>
          <dd>{{ summary.unknown }}</dd>
        </div>
      </dl>
      <div class="severity-row">
        <FpTag
          v-for="s in severities"
          :key="s.key"
          :severity="s.tone"
          :value="`${t(`inspection.summary.${s.key}`)} ${s.count}`"
          :class="{ 'is-zero': s.count === 0 }"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpTag from '@/components/ui/FpTag.vue'
import type { InspectionSummary } from '@/api/inspection'
import { scoreTone, type Tone } from '@/utils/inspection'

const props = defineProps<{ summary: InspectionSummary }>()

const { t } = useI18n()

const circumference = 2 * Math.PI * 52
const dashOffset = computed(
  () => circumference * (1 - Math.min(100, Math.max(0, props.summary.score)) / 100),
)
const tone = computed(() => scoreTone(props.summary.score))

const severities = computed<{ key: string; count: number; tone: Tone }[]>(() => [
  { key: 'critical', count: props.summary.critical, tone: 'danger' },
  { key: 'high', count: props.summary.high, tone: 'danger' },
  { key: 'medium', count: props.summary.medium, tone: 'warning' },
  { key: 'low', count: props.summary.low, tone: 'info' },
])
</script>

<style scoped>
.score-card {
  display: flex;
  align-items: center;
  gap: var(--fp-space-5);
  --score-color: var(--fp-success);
}
.score-card.is-warning {
  --score-color: var(--fp-warning);
}
.score-card.is-danger {
  --score-color: var(--fp-danger);
}
.score-ring {
  position: relative;
  flex: 0 0 132px;
  width: 132px;
  height: 132px;
}
.score-ring svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}
.ring-track {
  fill: none;
  stroke: color-mix(in oklch, var(--score-color) 14%, transparent);
  stroke-width: 10;
}
.ring-value {
  fill: none;
  stroke: var(--score-color);
  stroke-width: 10;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.6s ease;
}
.score-number {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
}
.score-value {
  font-family: var(--fp-font-title);
  font-size: 34px;
  font-weight: 700;
  line-height: 1;
  color: var(--score-color);
  font-variant-numeric: tabular-nums;
}
.score-grade {
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
}
.score-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.score-title {
  font-size: 14.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.score-stats {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--fp-space-3);
  margin: 0;
}
.stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.stat dt {
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
}
.stat dd {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
  line-height: 1.1;
  color: var(--fp-text-primary);
  font-variant-numeric: tabular-nums;
}
.stat.is-success dd {
  color: var(--fp-success);
}
.stat.is-danger dd {
  color: var(--fp-danger);
}
.stat.is-muted dd {
  color: var(--fp-text-secondary);
}
.severity-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
}
.severity-row :deep(.is-zero) {
  opacity: 0.45;
}
@media (max-width: 640px) {
  .score-card {
    flex-direction: column;
    align-items: stretch;
  }
  .score-ring {
    margin: 0 auto;
  }
}
</style>
