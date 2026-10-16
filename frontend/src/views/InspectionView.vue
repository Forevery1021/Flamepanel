<template>
  <LayoutContent :title="t('inspection.title')" :description="t('pageDesc.inspection')">
    <template #actions>
      <FpButton
        v-permission="{ perm: 'toolbox:execute', mode: 'view' }"
        variant="primary"
        icon="oi oi-refresh"
        :loading="running"
        :disabled="running"
        data-testid="inspection-run"
        @click="onRun"
      >
        {{ running ? t('inspection.running') : t('inspection.run') }}
      </FpButton>
    </template>

    <template v-if="report" #toolbar>
      <span class="meta-item mono" data-testid="inspection-generated">
        {{ t('inspection.generatedAt', { time: generatedAt }) }}
      </span>
      <span class="meta-item text-muted">
        {{
          cacheMinutes > 0
            ? t('inspection.cached', { minutes: cacheMinutes })
            : t('inspection.fresh')
        }}
      </span>
      <span class="meta-item text-muted">{{
        t('inspection.duration', { ms: report.duration_ms })
      }}</span>
      <span class="meta-item text-muted">
        {{ t('inspection.dataVersion', { version: report.data_version }) }}
      </span>
      <span class="meta-spacer" />
      <FpSelectButton
        :model-value="filter"
        :options="filterOptions"
        option-label="label"
        option-value="value"
        :allow-empty="false"
        size="small"
        @update:model-value="onFilterChange"
      />
    </template>

    <FpStatePanel
      :loading="loading && !report"
      :error="loadError"
      :empty="!loading && !loadError && !report"
      retryable
      :title="loadError ? t('inspection.loadFailed') : ''"
      :empty-title="t('inspection.empty')"
      @retry="refresh"
    >
      <template v-if="report">
        <div class="top-grid">
          <InspectionScoreCard :summary="report.summary" />
          <KernelInfoCard :kernel="report.kernel" />
        </div>

        <section class="panel urgent-panel" :class="{ 'is-clean': !urgent.length }">
          <header class="panel-header">
            <span class="panel-title">{{ t('inspection.urgent.title') }}</span>
            <FpTag
              :severity="urgent.length ? 'danger' : 'success'"
              :value="urgent.length ? String(urgent.length) : t('inspection.urgent.none')"
              dot
            />
          </header>
          <ul v-if="urgent.length" class="urgent-list" data-testid="inspection-urgent">
            <li v-for="f in urgent" :key="`${f.code}:${f.subject ?? ''}`" class="urgent-item">
              <FpTag
                :severity="severityTone(f.severity)"
                :value="t(`inspection.severity.${f.severity}`)"
              />
              <span class="urgent-title">{{
                t(findingI18nKey(f.code, 'title'), { code: f.code })
              }}</span>
              <code v-if="f.subject" class="urgent-subject">{{ f.subject }}</code>
              <span class="urgent-evidence mono">{{ f.evidence }}</span>
            </li>
          </ul>
        </section>

        <FpInlineMessage v-if="hasHeuristic" severity="info" class="heuristic-note">
          {{ t('inspection.heuristicNote') }}
        </FpInlineMessage>

        <FindingList :groups="groups" :fail-only="filter === 'fail'" />
      </template>
    </FpStatePanel>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import InspectionScoreCard from '@/components/inspection/InspectionScoreCard.vue'
import KernelInfoCard from '@/components/inspection/KernelInfoCard.vue'
import FindingList from '@/components/inspection/FindingList.vue'
import { useInspection } from '@/composables/useInspection'
import { cacheAgeMinutes, findingI18nKey, severityTone } from '@/utils/inspection'

/**
 * S-17 安全体检页：内核信息 + 风险 / 漏洞分析。
 *
 * 数据只有一份（`useInspection`，与仪表盘横幅共享 queryKey）；页面本身只做三件事：
 * 触发「立即体检」、切换「全部 / 仅风险项」、把报告分发给三块展示组件。
 */
const { t } = useI18n()
const toast = useFpToast()
const { report, groups, urgent, loading, error, refresh, run, running } = useInspection()

type Filter = 'all' | 'fail'
const filter = ref<Filter>('all')
const filterOptions = computed(() => [
  { value: 'all', label: t('inspection.filterAll') },
  { value: 'fail', label: t('inspection.filterFail') },
])
function onFilterChange(value: string | undefined) {
  if (value === 'all' || value === 'fail') filter.value = value
}

/** 已有报告时不用错误态打断页面（后台补刷失败保留旧报告） */
const loadError = computed(() => (report.value ? null : error.value))

const generatedAt = computed(() =>
  report.value ? new Date(report.value.generated_at).toLocaleString() : '',
)
const cacheMinutes = computed(() => (report.value ? cacheAgeMinutes(report.value) : 0))
const hasHeuristic = computed(() => report.value?.findings.some((f) => f.heuristic) ?? false)

async function onRun() {
  try {
    const next = await run()
    toast.success(
      t('inspection.runSuccess', { failed: next.summary.failed, score: next.summary.score }),
    )
  } catch (e) {
    toast.error(e, t('inspection.runFailed'))
  }
}
</script>

<style scoped>
.meta-item {
  font-size: var(--fp-font-xs);
}
.meta-spacer {
  flex: 1;
}
.top-grid {
  display: grid;
  grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);
  gap: var(--fp-space-4);
  margin-bottom: var(--fp-space-4);
}
.urgent-panel {
  margin-bottom: var(--fp-space-4);
  border-color: color-mix(in oklch, var(--fp-danger) 35%, var(--fp-border));
}
.urgent-panel.is-clean {
  border-color: var(--fp-border);
}
.urgent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.urgent-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 13px;
}
.urgent-title {
  font-weight: 600;
  color: var(--fp-text-primary);
}
.urgent-subject {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  padding: 1px 6px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-subtle);
  color: var(--fp-text-code);
}
.urgent-evidence {
  font-size: 12px;
  color: var(--fp-text-muted);
  word-break: break-all;
}
.heuristic-note {
  margin-bottom: var(--fp-space-4);
}
@media (max-width: 960px) {
  .top-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
