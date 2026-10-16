<template>
  <FpModal v-model="visible" :header="t('appStore.upgradeTitle', { name: app?.name ?? '' })">
    <FpStatePanel
      v-if="loading || loadError"
      :loading="loading"
      :error="loadError"
      retryable
      @retry="load"
    />
    <div v-else-if="preview" class="upgrade-body">
      <p class="upgrade-versions">
        <FpTag severity="neutral">{{ preview.from_version }}</FpTag>
        <i class="oi oi-arrow-right" aria-hidden="true" />
        <FpTag severity="info">{{ preview.to_version }}</FpTag>
      </p>
      <FpInlineMessage v-if="preview.up_to_date" severity="success">
        {{ t('appStore.upgradeUpToDate') }}
      </FpInlineMessage>
      <template v-else-if="merge">
        <FpInlineMessage v-if="!preview.base_available" severity="warn">
          {{ t('appStore.upgradeNoBase') }}
        </FpInlineMessage>
        <FpInlineMessage v-if="merge.reformatted" severity="info">
          {{ t('appStore.upgradeReformatted') }}
        </FpInlineMessage>
        <section v-if="merge.local_changes.length" class="upgrade-section">
          <h4>{{ t('appStore.upgradeLocalKept', { n: merge.local_changes.length }) }}</h4>
          <ul class="path-list">
            <li v-for="p in merge.local_changes" :key="'l' + p">
              <code>{{ p }}</code>
            </li>
          </ul>
        </section>
        <section v-if="merge.upstream_changes.length" class="upgrade-section">
          <h4>{{ t('appStore.upgradeUpstreamApplied', { n: merge.upstream_changes.length }) }}</h4>
          <ul class="path-list">
            <li v-for="p in merge.upstream_changes" :key="'u' + p">
              <code>{{ p }}</code>
            </li>
          </ul>
        </section>
        <section v-if="merge.conflicts.length" class="upgrade-section">
          <h4 class="conflict-title">
            {{ t('appStore.upgradeConflicts', { n: merge.conflicts.length }) }}
          </h4>
          <table class="conflict-table">
            <thead>
              <tr>
                <th scope="col">{{ t('appStore.upgradePath') }}</th>
                <th scope="col">{{ t('appStore.upgradeLocal') }}</th>
                <th scope="col">{{ t('appStore.upgradeUpstream') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="c in merge.conflicts" :key="c.path">
                <td>
                  <code>{{ c.path }}</code>
                </td>
                <td>
                  <code>{{ c.local ?? t('appStore.upgradeRemoved') }}</code>
                </td>
                <td>
                  <code>{{ c.upstream ?? t('appStore.upgradeRemoved') }}</code>
                </td>
              </tr>
            </tbody>
          </table>
          <fieldset class="strategy">
            <legend>{{ t('appStore.upgradeStrategy') }}</legend>
            <FpRadioGroup v-model="strategy" :options="strategyOptions" />
          </fieldset>
        </section>
        <p v-if="!hasChanges" class="upgrade-none">{{ t('appStore.upgradeNoChanges') }}</p>
      </template>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton
        variant="primary"
        :loading="upgrading"
        :disabled="!canUpgrade"
        @click="confirmUpgrade"
      >
        {{ t('appStore.upgrade') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpRadioGroup from '@/components/ui/FpRadioGroup.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorDetail } from '@/utils/error'
import {
  upgradeApp,
  upgradePreview,
  type ConflictStrategy,
  type InstalledApp,
  type UpgradePreview,
} from '@/api/appStore'

/**
 * 应用升级弹窗（Doc/30 M-C 三方合并）：先拉预览——列出保留的本地修改、生效的上游修改与
 * 冲突；有冲突时必须选择「保留本地 / 采用新版本」才能升级。`app` 为 null 时关闭。
 */
const props = defineProps<{ app: InstalledApp | null }>()
const emit = defineEmits<{ 'update:app': [InstalledApp | null]; done: [] }>()

const { t } = useI18n()
const toast = useFpToast()

const visible = computed({
  get: () => props.app !== null,
  set: (v: boolean) => {
    if (!v) emit('update:app', null)
  },
})
const preview = ref<UpgradePreview | null>(null)
const loading = ref(false)
const loadError = ref('')
const upgrading = ref(false)
const strategy = ref<ConflictStrategy | ''>('')

const merge = computed(() => preview.value?.merge ?? null)
const hasConflicts = computed(() => (merge.value?.conflicts.length ?? 0) > 0)
const hasChanges = computed(
  () =>
    !!merge.value &&
    merge.value.conflicts.length +
      merge.value.local_changes.length +
      merge.value.upstream_changes.length >
      0,
)
const canUpgrade = computed(
  () =>
    !!preview.value &&
    !preview.value.up_to_date &&
    !upgrading.value &&
    (!hasConflicts.value || strategy.value !== ''),
)
const strategyOptions = computed(() => [
  { label: t('appStore.upgradeKeepLocal'), value: 'local' },
  { label: t('appStore.upgradeTakeUpstream'), value: 'upstream' },
])

async function load() {
  const app = props.app
  if (!app) return
  loading.value = true
  loadError.value = ''
  try {
    preview.value = (await upgradePreview(app.id)).data
  } catch (e: unknown) {
    loadError.value = getErrorDetail(e) || t('appStore.upgradePreviewFailed')
  } finally {
    loading.value = false
  }
}

watch(
  () => props.app?.id,
  (id) => {
    preview.value = null
    strategy.value = ''
    if (id !== undefined) void load()
  },
  { immediate: true },
)

async function confirmUpgrade() {
  const app = props.app
  if (!app || !canUpgrade.value || !preview.value) return
  upgrading.value = true
  try {
    await upgradeApp(
      app.id,
      preview.value.to_version,
      hasConflicts.value ? (strategy.value as ConflictStrategy) : undefined,
    )
    toast.success(t('appStore.upgradeSuccess'))
    emit('update:app', null)
    emit('done')
  } catch (e: unknown) {
    toast.error(e, getErrorDetail(e) || t('appStore.upgradeFailed'))
  } finally {
    upgrading.value = false
  }
}
</script>

<style scoped>
.upgrade-body {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 44rem;
}
.upgrade-versions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
}
.upgrade-section h4 {
  margin: 0 0 0.375rem;
  font-size: 0.875rem;
}
.conflict-title {
  color: var(--fp-danger);
}
.path-list {
  margin: 0;
  padding-left: 1.25rem;
  max-height: 8rem;
  overflow: auto;
}
.conflict-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8125rem;
}
.conflict-table th,
.conflict-table td {
  border-bottom: 1px solid var(--fp-border);
  padding: 0.375rem 0.5rem;
  text-align: left;
  vertical-align: top;
  word-break: break-all;
}
.strategy {
  border: 0;
  margin: 0.75rem 0 0;
  padding: 0;
}
.strategy legend {
  margin-bottom: 0.375rem;
  font-weight: 600;
}
.upgrade-none {
  margin: 0;
  color: var(--fp-text-muted);
}
</style>
