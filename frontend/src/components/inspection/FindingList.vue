<template>
  <div class="finding-groups" data-testid="inspection-findings">
    <section v-for="group in visibleGroups" :key="group.category" class="panel finding-group">
      <header class="panel-header group-header">
        <span class="panel-title">{{ t(`inspection.category.${group.category}`) }}</span>
        <span class="group-counts">
          <FpTag
            v-if="group.failed"
            severity="danger"
            :value="`${t('inspection.status.fail')} ${group.failed}`"
          />
          <FpTag
            v-if="group.unknown"
            severity="neutral"
            :value="`${t('inspection.status.unknown')} ${group.unknown}`"
          />
          <span class="group-total">{{ group.findings.length }} {{ t('inspection.checks') }}</span>
        </span>
      </header>

      <ul class="finding-list">
        <li
          v-for="f in visible(group.findings)"
          :key="`${f.code}:${f.subject ?? ''}`"
          class="finding"
          :class="`is-${f.status}`"
          :data-code="f.code"
        >
          <div class="finding-main">
            <FpTag :severity="statusTone(f)" :value="t(`inspection.status.${f.status}`)" dot />
            <span class="finding-title">{{
              t(findingI18nKey(f.code, 'title'), { code: f.code })
            }}</span>
            <code v-if="f.subject" class="finding-subject">{{ f.subject }}</code>
            <span class="finding-spacer" />
            <FpTag
              v-if="f.status !== 'pass'"
              :severity="severityTone(f.severity)"
              :value="t(`inspection.severity.${f.severity}`)"
            />
            <FpTag v-if="f.heuristic" severity="neutral" :value="t('inspection.heuristic')" />
          </div>
          <div class="finding-detail">
            <span class="finding-label">{{ t('inspection.evidence') }}</span>
            <code class="finding-evidence">{{ f.evidence }}</code>
          </div>
          <div v-if="f.status !== 'pass'" class="finding-detail">
            <span class="finding-label">{{ t('inspection.advice') }}</span>
            <span class="finding-advice">{{
              t(findingI18nKey(f.code, 'advice'), { code: f.code })
            }}</span>
          </div>
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpTag from '@/components/ui/FpTag.vue'
import type { Finding } from '@/api/inspection'
import { findingI18nKey, severityTone, statusTone, type FindingGroup } from '@/utils/inspection'

const props = withDefaults(
  defineProps<{
    groups: FindingGroup[]
    /** 只显示 fail / unknown（隐藏通过项） */
    failOnly?: boolean
  }>(),
  { failOnly: false },
)

const { t } = useI18n()

function visible(findings: Finding[]): Finding[] {
  return props.failOnly ? findings.filter((f) => f.status !== 'pass') : findings
}

/** 仅风险项模式下隐藏没有任何风险 / 未知项的分类 */
const visibleGroups = computed(() =>
  props.failOnly ? props.groups.filter((g) => g.failed + g.unknown > 0) : props.groups,
)
</script>

<style scoped>
.finding-groups {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.group-header {
  gap: var(--fp-space-3);
}
.group-counts {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.group-total {
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
}
.finding-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}
.finding {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-1);
  padding: var(--fp-space-3) 0;
  border-top: 1px solid var(--fp-border);
}
.finding:first-child {
  border-top: 0;
}
.finding-main {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
}
.finding-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.finding.is-pass .finding-title {
  font-weight: 500;
  color: var(--fp-text-secondary);
}
.finding-subject {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  padding: 1px 6px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-subtle);
  color: var(--fp-text-code);
}
.finding-spacer {
  flex: 1;
}
.finding-detail {
  display: grid;
  grid-template-columns: 4em minmax(0, 1fr);
  gap: var(--fp-space-2);
  font-size: 12.5px;
  line-height: 1.5;
}
.finding-label {
  color: var(--fp-text-muted);
}
.finding-evidence {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  color: var(--fp-text-secondary);
  word-break: break-all;
}
.finding-advice {
  color: var(--fp-text-primary);
}
.finding.is-fail .finding-advice {
  color: var(--fp-text-primary);
}
</style>
