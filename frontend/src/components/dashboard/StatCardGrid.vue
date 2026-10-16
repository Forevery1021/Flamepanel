<template>
  <div class="stats-grid" :class="{ 'is-pending': pending }" :aria-busy="pending || undefined">
    <button
      v-for="s in cards"
      :key="s.key"
      class="stat-card"
      :class="[`stat-${s.key}`, { 'is-clickable': s.to }]"
      type="button"
      :aria-label="s.to ? t('dashboard.goTo', { module: s.label }) : s.label"
      @click="s.to && emit('navigate', s.to)"
    >
      <span class="stat-top">
        <span class="stat-label">
          <i class="oi stat-label__icon" :class="s.icon" aria-hidden="true" />
          {{ s.label }}
        </span>
        <svg
          v-if="sparks[s.key]"
          class="stat-spark"
          viewBox="0 0 100 28"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <path
            :d="sparks[s.key]"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />
        </svg>
      </span>
      <span class="stat-value" :class="`tone-${tones[s.key]}`">
        <span v-if="pending" class="stat-skeleton" aria-hidden="true" />
        <template v-else>{{ s.value }}</template>
      </span>
      <span
        class="stat-bar"
        role="progressbar"
        :aria-valuenow="s.percent"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-label="s.label"
      >
        <span
          class="stat-bar__fill"
          :class="`tone-${tones[s.key]}`"
          :style="{ width: `${Math.max(2, Math.min(100, s.percent))}%` }"
        />
      </span>
      <span class="stat-detail">
        <span class="stat-detail__text">{{ pending ? t('dashboard.waitingData') : s.detail }}</span>
        <i v-if="s.to" class="oi oi-arrow-right stat-go" aria-hidden="true" />
      </span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { sparkPath, statTone, type DashboardStatCard, type StatTone } from '@/utils/dashboardFormat'
import type { MetricsSnapshot } from '@/types'

/**
 * 指标卡网格（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * **纯展示 + 跳转上抛**：卡片题面（数值/百分比/明细）由 `buildStatCards` 纯函数
 * 拼装后作为 prop 传入，本组件只负责渲染与语义。
 *
 * 两处刻意保留的细节：
 * - 卡片是 `<button>` 而不是 `<div @click>`（键盘可达 + `aria-label` 说明去向）；
 *   无跳转目标的卡片置 `cursor: default` 且不可聚焦敲击——它只是「展示」。
 * - `aria-label` 走 `dashboard.goTo` 插值（`查看{module}详情`），图标 `oi-chevron-right`
 *   标 `aria-hidden`（纯装饰，读屏重复念「右箭头」没有意义）。
 */
const props = withDefaults(
  defineProps<{
    cards: DashboardStatCard[]
    /** 近 N 点历史（用于迷你趋势线） */
    history?: MetricsSnapshot[]
    /** 尚未收到首帧：数值位显示骨架条而不是误导性的 0%（开屏优化） */
    pending?: boolean
  }>(),
  { history: () => [], pending: false },
)

const emit = defineEmits<{ navigate: [to: string] }>()

const { t } = useI18n()

const SPARK_POINTS = 30
const FIELD: Record<string, keyof MetricsSnapshot> = {
  cpu: 'cpu_usage',
  mem: 'memory_usage_percent',
  disk: 'disk_usage_percent',
  load: 'load_one',
}

/**
 * 四条迷你趋势线一次算完（开屏优化）：旧实现在模板里对每张卡调用两次 `spark()`
 * （v-if 与 path 各一次）再调 `sparkPath()`，每个 tick 做 8 次 slice/map；
 * 现在历史引用变化时只算一次，模板按 key 取值。
 */
const sparks = computed<Record<string, string>>(() => {
  const h = props.history
  const win = h.length > SPARK_POINTS ? h.slice(-SPARK_POINTS) : h
  const out: Record<string, string> = {}
  for (const key of Object.keys(FIELD)) {
    const f = FIELD[key]
    const d = sparkPath(win.map((x) => Number(x[f]) || 0))
    if (d) out[key] = d
  }
  return out
})

const tones = computed<Record<string, StatTone>>(() => {
  const out: Record<string, StatTone> = {}
  for (const c of props.cards) out[c.key] = statTone(c.percent, c.key)
  return out
})
</script>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--fp-space-4);
}
.stat-card {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  padding: var(--fp-space-4) var(--fp-space-5);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-surface);
  backdrop-filter: var(--fp-surface-filter);
  -webkit-backdrop-filter: var(--fp-surface-filter);
  border: 1px solid var(--fp-border);
  text-align: left;
  font: inherit;
  color: inherit;
  cursor: default;
  min-width: 0;
  transition:
    transform 120ms var(--fp-ease-out),
    border-color var(--fp-transition-fast);
}
.stat-card.is-clickable {
  cursor: pointer;
}
.stat-card.is-clickable:hover {
  transform: translateY(-1px);
  border-color: var(--fp-border-strong);
}
.stat-card.is-clickable:focus-visible {
  outline: 2px solid var(--fp-brand);
  outline-offset: 2px;
}
.stat-card.is-clickable:active {
  transform: translateY(0);
}
.stat-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fp-space-3);
  min-height: 28px;
}
.stat-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fp-font-xs);
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--fp-text-secondary);
}
.stat-label__icon {
  font-size: 13px;
  color: var(--fp-text-muted);
}
.stat-spark {
  width: 84px;
  height: 28px;
  color: var(--fp-text-muted);
  opacity: 0.8;
}
.stat-value {
  font-size: var(--fp-font-2xl);
  font-weight: 700;
  line-height: 1.1;
  letter-spacing: -0.03em;
  font-variant-numeric: tabular-nums;
  color: var(--fp-text-primary);
}
.stat-skeleton {
  display: inline-block;
  width: 3.5em;
  height: 0.9em;
  border-radius: 6px;
  background: linear-gradient(
    90deg,
    var(--fp-bg-hover) 25%,
    var(--fp-border) 50%,
    var(--fp-bg-hover) 75%
  );
  background-size: 200% 100%;
  animation: stat-shimmer 1.4s ease-in-out infinite;
  vertical-align: middle;
}
@keyframes stat-shimmer {
  from {
    background-position: 200% 0;
  }
  to {
    background-position: -200% 0;
  }
}
.stat-bar {
  display: block;
  height: 4px;
  border-radius: 999px;
  background: var(--fp-bg-hover);
  overflow: hidden;
}
.stat-bar__fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  transition: width 600ms var(--fp-ease-out);
}
.tone-ok.stat-bar__fill {
  background: var(--fp-success);
}
.tone-warn.stat-bar__fill {
  background: var(--fp-warning);
}
.tone-bad.stat-bar__fill {
  background: var(--fp-danger);
}
.stat-value.tone-bad {
  color: var(--fp-danger);
}
.stat-detail {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fp-space-2);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
  font-variant-numeric: tabular-nums;
}
.stat-detail__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.stat-go {
  font-size: 12px;
  flex-shrink: 0;
  opacity: 0;
  transform: translateX(-4px);
  transition:
    opacity var(--fp-transition-fast),
    transform var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.stat-card.is-clickable:hover .stat-go,
.stat-card.is-clickable:focus-visible .stat-go {
  opacity: 1;
  transform: none;
  color: var(--fp-brand);
}
@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (max-width: 600px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
@media (prefers-reduced-motion: reduce) {
  .stat-card,
  .stat-bar__fill,
  .stat-go {
    transition: none;
  }
  .stat-skeleton {
    animation: none;
  }
}
</style>
