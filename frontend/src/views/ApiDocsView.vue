<template>
  <LayoutContent
    :title="t('apiDocs.title')"
    :description="t('pageDesc.apiDocs')"
    :summary="summary"
    reload
    @reload="load"
  >
    <template #actions>
      <FpButton
        variant="secondary"
        icon="oi oi-download"
        :label="t('apiDocs.downloadSpec')"
        :disabled="!doc"
        @click="downloadSpec"
      />
    </template>

    <FpStatePanel
      :loading="loading"
      :error="error"
      :empty="!loading && !error && operations.length === 0"
      :empty-title="t('apiDocs.empty')"
      empty-icon="oi oi-code"
      retryable
      @retry="load"
    >
      <div class="api-docs">
        <nav class="api-docs__tags" :aria-label="t('apiDocs.modules')">
          <button
            type="button"
            class="api-docs__tag"
            :class="{ 'is-active': !activeTag }"
            :aria-pressed="!activeTag"
            @click="activeTag = ''"
          >
            <span>{{ t('apiDocs.allModules') }}</span>
            <span class="api-docs__count">{{ searched.length }}</span>
          </button>
          <button
            v-for="g in tagCounts"
            :key="g.tag"
            type="button"
            class="api-docs__tag"
            :class="{ 'is-active': activeTag === g.tag, 'is-dim': g.count === 0 }"
            :aria-pressed="activeTag === g.tag"
            @click="activeTag = g.tag"
          >
            <span>{{ g.tag }}</span>
            <span class="api-docs__count">{{ g.count }}</span>
          </button>
        </nav>

        <div class="api-docs__main">
          <div class="api-docs__toolbar">
            <FpInput
              ref="searchRef"
              v-model="query"
              class="api-docs__search"
              :placeholder="t('apiDocs.searchPlaceholder')"
              :aria-label="t('apiDocs.search')"
            />
            <FpSelectButton
              v-model="method"
              :options="methodOptions"
              option-label="label"
              option-value="value"
              :allow-empty="false"
              size="small"
            />
          </div>

          <p class="api-docs__stats" aria-live="polite">
            {{ t('apiDocs.showing', { shown: visible.length, total: operations.length }) }}
            <span v-if="doc?.info?.version"> · v{{ doc.info.version }}</span>
          </p>

          <FpEmpty
            v-if="visible.length === 0"
            icon="oi oi-search"
            :title="t('apiDocs.noMatch')"
            :description="t('apiDocs.noMatchHint')"
          />

          <section v-for="g in groups" :key="g.tag" class="api-docs__group">
            <h3 class="api-docs__group-title">
              {{ g.tag }}
              <span class="api-docs__count">{{ g.operations.length }}</span>
            </h3>
            <TransitionGroup name="api-op-list" tag="div" class="api-docs__ops">
              <ApiOperationCard
                v-for="op in g.operations"
                :key="op.key"
                :op="op"
                :doc="doc!"
                :origin="origin"
                :open="openKey === op.key"
                @toggle="toggle(op.key)"
              />
            </TransitionGroup>
          </section>
        </div>
      </div>
    </FpStatePanel>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpEmpty from '@/components/ui/FpEmpty.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import ApiOperationCard from '@/components/apiDocs/ApiOperationCard.vue'
import { getOpenApiDocument } from '@/api/openapiDoc'
import {
  HTTP_METHODS,
  filterOperations,
  groupByTag,
  listOperations,
  type ApiOperation,
  type OpenApiDocument,
} from '@/utils/openapi'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const doc = shallowRef<OpenApiDocument | null>(null)
const operations = shallowRef<ApiOperation[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const origin = typeof window === 'undefined' ? '' : window.location.origin

async function load() {
  loading.value = true
  error.value = null
  try {
    const { data } = await getOpenApiDocument()
    doc.value = data
    operations.value = listOperations(data)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

// ── 与 URL 同步的筛选状态（刷新 / 分享链接保持位置） ────────────────────────
function queryString(key: string): string {
  const v = route.query[key]
  return typeof v === 'string' ? v : ''
}
function setQuery(key: string, value: string) {
  void router.replace({ query: { ...route.query, [key]: value || undefined } })
}

const query = computed<string>({
  get: () => queryString('q'),
  set: (v) => setQuery('q', v.trim() ? v : ''),
})
const activeTag = computed<string>({
  get: () => queryString('tag'),
  set: (v) => setQuery('tag', v),
})
const method = computed<string>({
  get: () =>
    (HTTP_METHODS as readonly string[]).includes(queryString('method'))
      ? queryString('method')
      : 'all',
  set: (v) => setQuery('method', v === 'all' ? '' : v),
})
const openKey = computed(() => queryString('op'))

function toggle(key: string) {
  setQuery('op', openKey.value === key ? '' : key)
}

const methodOptions = computed(() => [
  { label: t('apiDocs.allMethods'), value: 'all' },
  ...HTTP_METHODS.map((m) => ({ label: m.toUpperCase(), value: m })),
])

/** 搜索 + 方法过滤（不含模块）——左侧模块计数基于它 */
const searched = computed(() => {
  const byQuery = filterOperations(operations.value, query.value)
  return method.value === 'all' ? byQuery : byQuery.filter((op) => op.method === method.value)
})
const visible = computed(() =>
  activeTag.value ? searched.value.filter((op) => op.tag === activeTag.value) : searched.value,
)
const groups = computed(() => groupByTag(visible.value))
const tagCounts = computed(() => {
  const counts = new Map<string, number>()
  for (const op of operations.value) counts.set(op.tag, 0)
  for (const op of searched.value) counts.set(op.tag, (counts.get(op.tag) ?? 0) + 1)
  return [...counts.entries()].map(([tag, count]) => ({ tag, count }))
})

const summary = computed(() => {
  const ops = operations.value
  if (ops.length === 0) return []
  return [
    { label: t('apiDocs.endpoints'), value: ops.length },
    { label: t('apiDocs.modules'), value: new Set(ops.map((o) => o.tag)).size },
    { label: t('apiDocs.publicEndpoints'), value: ops.filter((o) => !o.secured).length },
  ]
})

function downloadSpec() {
  if (!doc.value) return
  const blob = new Blob([JSON.stringify(doc.value, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `flamepanel-openapi-${doc.value.info?.version ?? 'latest'}.json`
  a.click()
  URL.revokeObjectURL(url)
}

// 深链：带 `?op=` 打开时滚动到该端点
watch(
  () => [openKey.value, operations.value.length] as const,
  async ([key, n], prev) => {
    if (!key || n === 0 || (prev && prev[0] === key && prev[1] === n)) return
    await nextTick()
    const el = document.querySelector('.api-op.is-open')
    el?.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' })
  },
)

// `/` 聚焦搜索框（与内核日志页一致）
const searchRef = ref<InstanceType<typeof FpInput> | null>(null)
function onKey(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null
  if (e.key !== '/' || target?.closest('input, textarea, [contenteditable="true"]')) return
  e.preventDefault()
  ;(searchRef.value?.$el as HTMLElement | undefined)?.querySelector('input')?.focus()
}

onMounted(() => {
  void load()
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<style scoped>
.api-docs {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: var(--fp-space-4);
  align-items: start;
}
.api-docs__tags {
  position: sticky;
  top: var(--fp-space-4);
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: calc(100vh - var(--fp-header-height) - var(--fp-space-8));
  overflow-y: auto;
}
.api-docs__tag {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fp-space-2);
  padding: 6px 10px;
  border: 0;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: var(--fp-text-secondary);
  font: inherit;
  font-size: var(--fp-font-sm);
  text-align: left;
  cursor: pointer;
  transition:
    background var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.api-docs__tag:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.api-docs__tag:focus-visible {
  outline: none;
  box-shadow: var(--fp-focus-ring);
}
.api-docs__tag.is-active {
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  font-weight: 600;
}
.api-docs__tag.is-dim {
  opacity: 0.45;
}
.api-docs__count {
  min-width: 24px;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--fp-bg-subtle);
  color: var(--fp-text-muted);
  font-size: var(--fp-font-xs);
  font-weight: 500;
  text-align: center;
}
.api-docs__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-3);
}
.api-docs__search {
  flex: 1;
  min-width: 220px;
}
.api-docs__stats {
  margin: var(--fp-space-2) 0 var(--fp-space-3);
  color: var(--fp-text-muted);
  font-size: var(--fp-font-xs);
}
.api-docs__group + .api-docs__group {
  margin-top: var(--fp-space-5);
}
.api-docs__group-title {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin: 0 0 var(--fp-space-2);
  color: var(--fp-text-primary);
  font-size: var(--fp-font-md);
}
.api-docs__ops {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.api-op-list-enter-active,
.api-op-list-leave-active {
  transition:
    opacity var(--fp-transition-fast),
    transform var(--fp-transition-fast);
}
.api-op-list-enter-from,
.api-op-list-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
@media (prefers-reduced-motion: reduce) {
  .api-op-list-enter-active,
  .api-op-list-leave-active {
    transition: none;
  }
}
@media (max-width: 900px) {
  .api-docs {
    grid-template-columns: 1fr;
  }
  .api-docs__tags {
    position: static;
    flex-direction: row;
    flex-wrap: wrap;
    max-height: none;
  }
}
</style>
