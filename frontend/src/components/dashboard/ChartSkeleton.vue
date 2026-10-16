<template>
  <div class="panel chart-skeleton" :style="{ minHeight: `${height}px` }" aria-hidden="true">
    <div class="chart-skeleton__title" />
    <div class="chart-skeleton__body" />
  </div>
</template>

<script setup lang="ts">
/**
 * 图表面板骨架（开屏优化）：异步图表块下载/解析期间占住同样的版位，
 * 避免图表就绪时页面跳动（CLS）。高度由调用方按真实面板给定。
 */
withDefaults(defineProps<{ height?: number }>(), { height: 340 })
</script>

<style scoped>
.chart-skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.chart-skeleton__title,
.chart-skeleton__body {
  border-radius: var(--fp-radius-sm);
  background: linear-gradient(
    90deg,
    var(--fp-bg-hover) 25%,
    var(--fp-border) 50%,
    var(--fp-bg-hover) 75%
  );
  background-size: 200% 100%;
  animation: chart-shimmer 1.4s ease-in-out infinite;
}
.chart-skeleton__title {
  width: 120px;
  height: 14px;
}
.chart-skeleton__body {
  flex: 1;
  min-height: 120px;
  opacity: 0.6;
}
@keyframes chart-shimmer {
  from {
    background-position: 200% 0;
  }
  to {
    background-position: -200% 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .chart-skeleton__title,
  .chart-skeleton__body {
    animation: none;
  }
}
</style>
