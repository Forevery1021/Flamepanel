<template>
  <div class="klog">
    <FpInlineMessage v-if="overview && !enabled" severity="info">
      {{ t('log.kernel.disabled') }}
    </FpInlineMessage>

    <template v-else>
      <!-- 第一行：文件 / 跟随 / 操作 -->
      <div class="klog-bar">
        <div class="klog-bar__file">
          <FpSelect
            v-model="file"
            :options="fileOptions"
            option-label="label"
            option-value="value"
            :placeholder="t('log.kernel.file')"
            :aria-label="t('log.kernel.file')"
          />
        </div>
        <FpSwitch
          v-model="following"
          :label="t('log.kernel.follow')"
          :disabled="!isViewingCurrent"
        />
        <span v-if="following" class="klog-live" aria-live="polite">
          <span class="klog-live__dot" />{{ t('log.kernel.live') }}
        </span>
        <div class="klog-bar__spacer" />
        <div v-permission="{ perm: 'settings:update', mode: 'view' }" class="klog-bar__level">
          <FpSelect
            :model-value="overview?.level"
            :options="runtimeLevelOptions"
            option-label="label"
            option-value="value"
            :aria-label="t('log.kernel.runtimeLevel')"
            @update:model-value="onChangeLevel"
          />
        </div>
        <FpButton
          variant="ghost"
          size="small"
          icon="oi oi-refresh"
          :loading="loading"
          :title="t('common.refresh')"
          :aria-label="t('common.refresh')"
          @click="refresh"
        />
        <FpButton
          variant="ghost"
          size="small"
          icon="oi oi-download"
          :disabled="!file"
          :title="t('log.kernel.download')"
          :aria-label="t('log.kernel.download')"
          @click="onDownload"
        />
        <FpButton
          v-permission="{ perm: 'log:delete', mode: 'view' }"
          variant="ghost"
          size="small"
          icon="oi oi-trash"
          :disabled="!file || isViewingCurrent"
          :title="isViewingCurrent ? t('log.kernel.cannotDeleteCurrent') : t('log.kernel.delete')"
          :aria-label="t('log.kernel.delete')"
          @click="onDelete"
        />
      </div>

      <!-- 第二行：五级开关（带计数）+ 搜索 -->
      <div class="klog-filters">
        <div class="klog-levels" role="group" :aria-label="t('log.level')">
          <button
            type="button"
            class="klog-level klog-level--all"
            :class="{ 'is-active': !levels.length }"
            :aria-pressed="!levels.length"
            @click="clearLevels"
          >
            {{ t('log.all') }}
            <span v-if="counts" class="klog-level__count">{{ formatCount(counts.total) }}</span>
          </button>
          <button
            v-for="lv in LOG_LEVELS"
            :key="lv"
            type="button"
            class="klog-level"
            :class="[`klog-level--${lv}`, { 'is-active': levels.includes(lv) }]"
            :aria-pressed="levels.includes(lv)"
            @click="toggleLevel(lv)"
          >
            <span class="klog-level__dot" />
            {{ t(`log.kernel.levels.${lv}`) }}
            <span v-if="counts" class="klog-level__count">{{ formatCount(counts[lv]) }}</span>
          </button>
          <span
            v-if="countsTruncated"
            v-tooltip="t('log.kernel.countsTruncated')"
            class="klog-level__approx"
            >≈</span
          >
        </div>
        <div class="klog-search">
          <FpInput
            ref="searchRef"
            v-model="keyword"
            :placeholder="t('log.kernel.searchPlaceholder')"
            :aria-label="t('log.kernel.searchPlaceholder')"
          />
        </div>
      </div>

      <FpInlineMessage v-if="overview && overview.dropped_lines > 0" severity="warn">
        {{ t('log.kernel.dropped', { n: overview.dropped_lines }) }}
      </FpInlineMessage>

      <FpStatePanel
        :loading="loading && !rows.length"
        :error="errorText"
        :empty="!rows.length"
        skeleton
        :skeleton-rows="8"
        retryable
        :empty-title="t('common.noData')"
        :empty-desc="hasFilter ? t('log.kernel.emptyFiltered') : t('log.kernel.empty')"
        @retry="refresh"
      >
        <ul class="klog-list" :class="{ 'is-dim': loading }" role="list">
          <li
            v-for="row in rows"
            :key="row.key"
            class="klog-row"
            :class="[
              `klog-row--${row.level}`,
              { 'is-open': openKey === row.key, 'is-fresh': freshKeys.has(row.key) },
            ]"
          >
            <button
              type="button"
              class="klog-row__main"
              :aria-expanded="openKey === row.key"
              @click="toggleRow(row.key)"
            >
              <span class="klog-row__time" :title="row.timestamp">{{
                shortTime(row.timestamp)
              }}</span>
              <span class="klog-row__level">{{ row.level.toUpperCase() }}</span>
              <span class="klog-row__target" :title="row.target">{{
                shortTarget(row.target)
              }}</span>
              <span class="klog-row__msg">
                <template v-for="(seg, i) in highlight(row.message)" :key="i">
                  <mark v-if="seg.hit">{{ seg.text }}</mark>
                  <template v-else>{{ seg.text }}</template>
                </template>
              </span>
              <i class="oi oi-chevron-down klog-row__chevron" aria-hidden="true" />
            </button>
            <div v-if="openKey === row.key" class="klog-detail">
              <dl class="klog-detail__meta">
                <dt>{{ t('log.time') }}</dt>
                <dd class="mono">{{ row.timestamp || '—' }}</dd>
                <dt>{{ t('log.kernel.target') }}</dt>
                <dd class="mono">{{ row.target || '—' }}</dd>
                <template v-if="row.request_id">
                  <dt>{{ t('log.kernel.requestId') }}</dt>
                  <dd class="mono klog-detail__rid">
                    {{ row.request_id }}
                    <CopyButton :text="row.request_id" />
                    <FpButton variant="link" size="small" @click="keyword = row.request_id">
                      {{ t('log.kernel.filterByRequest') }}
                    </FpButton>
                  </dd>
                </template>
              </dl>
              <div class="klog-detail__json">
                <pre class="mono">{{ prettyFields(row) }}</pre>
                <CopyButton :text="copyText(row)" />
              </div>
            </div>
          </li>
        </ul>
        <div class="klog-footer">
          <FpButton
            v-if="hasMore"
            variant="secondary"
            size="small"
            :loading="loadingMore"
            @click="loadMore"
          >
            {{ t('log.kernel.loadMore') }}
          </FpButton>
          <span v-else-if="rows.length" class="text-xs text-muted">{{ t('log.kernel.end') }}</span>
          <span class="text-xs text-muted klog-footer__hint">{{ t('log.kernel.shortcut') }}</span>
        </div>
      </FpStatePanel>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import CopyButton from '@/components/ui/CopyButton.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useFpToast } from '@/components/ui/FpToast'
import { LOG_LEVELS, type LogLevel } from '@/api/kernelLogs'
import { useKernelLogs, type KernelLogRow } from '@/composables/useKernelLogs'
import { getErrorMessage } from '@/utils/error'
import { formatSize } from '@/utils/format'

const props = defineProps<{ initialQuery?: string }>()

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const {
  overview,
  enabled,
  files,
  file,
  isViewingCurrent,
  levels,
  keyword,
  rows,
  counts,
  countsTruncated,
  loading,
  loadingMore,
  hasMore,
  error,
  following,
  freshKeys,
  refresh,
  loadMore,
  toggleLevel,
  clearLevels,
  download,
  removeFile,
  changeLevel,
} = useKernelLogs({ q: props.initialQuery })

const openKey = ref<string | null>(null)
const searchRef = ref<{ $el?: HTMLElement } | null>(null)

const fileOptions = computed(() =>
  files.value.map((f) => ({
    value: f.name,
    label: `${f.name} · ${formatSize(f.size)}${f.current ? ` · ${t('log.kernel.current')}` : ''}`,
  })),
)

const runtimeLevelOptions = computed(() =>
  LOG_LEVELS.map((lv) => ({
    value: lv,
    label: `${t('log.kernel.runtimeLevel')}: ${t(`log.kernel.levels.${lv}`)}`,
  })),
)

const hasFilter = computed(() => levels.value.length > 0 || keyword.value.trim() !== '')
const errorText = computed(() =>
  error.value && !rows.value.length
    ? getErrorMessage(error.value, t('log.kernel.loadFailed'))
    : null,
)

function toggleRow(key: string) {
  openKey.value = openKey.value === key ? null : key
}

function formatCount(n: number): string {
  return n >= 10000 ? `${(n / 1000).toFixed(n >= 100000 ? 0 : 1)}k` : String(n)
}

/** `2026-09-25T10:00:00.123456Z` → `09-25 10:00:00.123`（本地时区） */
function shortTime(ts: string): string {
  if (!ts) return '—'
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  const p = (n: number, w = 2) => String(n).padStart(w, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}.${p(d.getMilliseconds(), 3)}`
}

/** `flame_kernel::api::handler::toolbox` → `handler::toolbox` */
function shortTarget(target: string): string {
  const parts = target.split('::')
  return parts.length > 2 ? parts.slice(-2).join('::') : target
}

/** 关键字高亮（纯文本分段，不使用 v-html） */
function highlight(text: string): { text: string; hit: boolean }[] {
  const q = keyword.value.trim()
  if (!q) return [{ text, hit: false }]
  const lower = text.toLowerCase()
  const needle = q.toLowerCase()
  const out: { text: string; hit: boolean }[] = []
  let from = 0
  for (let i = lower.indexOf(needle); i >= 0; i = lower.indexOf(needle, from)) {
    if (i > from) out.push({ text: text.slice(from, i), hit: false })
    out.push({ text: text.slice(i, i + needle.length), hit: true })
    from = i + needle.length
  }
  if (from < text.length) out.push({ text: text.slice(from), hit: false })
  return out
}

function prettyFields(row: KernelLogRow): string {
  return JSON.stringify(row.fields ?? {}, null, 2)
}

function copyText(row: KernelLogRow): string {
  return JSON.stringify({
    timestamp: row.timestamp,
    level: row.level,
    target: row.target,
    message: row.message,
    request_id: row.request_id,
    fields: row.fields,
  })
}

async function onDownload() {
  try {
    await download()
  } catch (e) {
    toast.error(e, t('log.kernel.downloadFailed'))
  }
}

function onDelete() {
  const name = file.value
  if (!name) return
  confirmAction({
    header: t('log.kernel.delete'),
    message: t('log.kernel.deleteConfirm', { name }),
    acceptLabel: t('common.delete'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await removeFile(name)
        toast.success(t('log.kernel.deleted'))
      } catch (e) {
        toast.error(e, t('log.kernel.deleteFailed'))
      }
    },
  })
}

async function onChangeLevel(v: unknown) {
  if (typeof v !== 'string' || !v || v === overview.value?.level) return
  try {
    const lv = await changeLevel(v as LogLevel)
    toast.success(t('log.kernel.levelChanged', { level: t(`log.kernel.levels.${lv}`) }))
  } catch (e) {
    toast.error(e, t('log.kernel.levelChangeFailed'))
  }
}

/** `/` 聚焦搜索框（输入控件内不拦截） */
function onKey(e: KeyboardEvent) {
  if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey) return
  const el = e.target as HTMLElement | null
  if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable)) return
  const input = searchRef.value?.$el?.querySelector?.('input')
  if (input) {
    e.preventDefault()
    input.focus()
  }
}

// 深链在 keepAlive 缓存下再次进入时也要生效
watch(
  () => props.initialQuery,
  (q) => {
    if (q != null && q !== keyword.value) keyword.value = q
  },
)

onMounted(() => {
  void refresh()
  window.addEventListener('keydown', onKey)
})
onActivated(() => window.addEventListener('keydown', onKey))
onDeactivated(() => window.removeEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<style scoped>
.klog {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.klog-bar,
.klog-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2) var(--fp-space-3);
}
.klog-bar__file {
  width: min(360px, 100%);
}
.klog-bar__level {
  width: 190px;
}
.klog-bar__spacer {
  flex: 1;
}
.klog-live {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fp-font-size-xs, 12px);
  color: var(--fp-success);
}
.klog-live__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--fp-success);
  animation: klog-pulse 1.6s ease-in-out infinite;
}
.klog-search {
  flex: 1;
  min-width: 220px;
}

/* 五级开关 */
.klog-levels {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.klog-level {
  --lv: var(--fp-text-secondary);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border: 1px solid var(--fp-border);
  border-radius: 999px;
  background: transparent;
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-size-sm, 13px);
  cursor: pointer;
  transition:
    background-color 0.15s ease,
    border-color 0.15s ease,
    color 0.15s ease,
    transform 0.1s ease;
}
.klog-level:hover {
  border-color: var(--lv);
  color: var(--fp-text-primary);
}
.klog-level:active {
  transform: scale(0.96);
}
.klog-level:focus-visible {
  outline: 2px solid var(--fp-brand);
  outline-offset: 2px;
}
.klog-level.is-active {
  border-color: var(--lv);
  background: color-mix(in srgb, var(--lv) 14%, transparent);
  color: var(--fp-text-primary);
}
.klog-level__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--lv);
}
.klog-level__count {
  font-variant-numeric: tabular-nums;
  color: var(--fp-text-muted);
}
.klog-level__approx {
  color: var(--fp-text-muted);
  cursor: help;
}
.klog-level--all {
  --lv: var(--fp-brand);
}
.klog-level--trace,
.klog-row--trace {
  --lv: var(--fp-text-muted);
}
.klog-level--debug,
.klog-row--debug {
  --lv: #8b5cf6;
}
.klog-level--info,
.klog-row--info {
  --lv: var(--fp-info, #3b82f6);
}
.klog-level--warn,
.klog-row--warn {
  --lv: var(--fp-warning);
}
.klog-level--error,
.klog-row--error {
  --lv: var(--fp-danger);
}

/* 列表 */
.klog-list {
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md, 8px);
  overflow: hidden;
  transition: opacity 0.15s ease;
}
.klog-list.is-dim {
  opacity: 0.6;
}
.klog-row {
  border-left: 3px solid var(--lv);
  border-bottom: 1px solid var(--fp-border);
  /*
   * 长列表渲染优化（跟随模式最多 2000 行）：行高可变且可展开，不适合固定 itemSize 的虚拟滚动；
   * 改用 content-visibility —— 视口外的行跳过布局与绘制，`auto` 记住已渲染行的真实高度，
   * 滚动条不跳动。展开行除外（详情高度差异大，保持常规渲染）。
   */
  content-visibility: auto;
  contain-intrinsic-size: auto 34px;
}
.klog-row.is-open {
  content-visibility: visible;
}
.klog-row:last-child {
  border-bottom: none;
}
.klog-row--error {
  background: color-mix(in srgb, var(--fp-danger) 5%, transparent);
}
.klog-row--warn {
  background: color-mix(in srgb, var(--fp-warning) 4%, transparent);
}
.klog-row.is-fresh {
  animation: klog-fresh 1.8s ease-out;
}
.klog-row__main {
  display: grid;
  grid-template-columns: 150px 56px minmax(80px, 180px) 1fr 16px;
  align-items: center;
  gap: var(--fp-space-3);
  width: 100%;
  padding: 6px var(--fp-space-3);
  border: none;
  background: transparent;
  color: var(--fp-text-primary);
  font-size: var(--fp-font-size-sm, 13px);
  text-align: left;
  cursor: pointer;
}
.klog-row__main:hover {
  background: var(--fp-bg-hover, rgba(127, 127, 127, 0.08));
}
.klog-row__main:focus-visible {
  outline: 2px solid var(--fp-brand);
  outline-offset: -2px;
}
.klog-row__time,
.klog-row__target {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  color: var(--fp-text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.klog-row__level {
  font-family: var(--fp-font-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--lv);
}
.klog-row__msg {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.klog-row.is-open .klog-row__msg {
  white-space: normal;
  word-break: break-word;
}
.klog-row__msg mark {
  padding: 0 1px;
  border-radius: 2px;
  background: color-mix(in srgb, var(--fp-warning) 35%, transparent);
  color: inherit;
}
.klog-row__chevron {
  font-size: 11px;
  color: var(--fp-text-muted);
  transition: transform 0.15s ease;
}
.klog-row.is-open .klog-row__chevron {
  transform: rotate(180deg);
}
.klog-detail {
  padding: var(--fp-space-2) var(--fp-space-3) var(--fp-space-3) calc(150px + var(--fp-space-6));
  animation: klog-open 0.15s ease-out;
}
.klog-detail__meta {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 4px var(--fp-space-3);
  margin: 0 0 var(--fp-space-2);
  font-size: 12px;
}
.klog-detail__meta dt {
  color: var(--fp-text-muted);
}
.klog-detail__meta dd {
  margin: 0;
  word-break: break-all;
}
.klog-detail__rid {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
}
.klog-detail__json {
  position: relative;
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
}
.klog-detail__json pre {
  flex: 1;
  max-height: 320px;
  margin: 0;
  padding: var(--fp-space-2) var(--fp-space-3);
  overflow: auto;
  border-radius: var(--fp-radius-sm, 4px);
  background: var(--fp-bg-subtle, rgba(127, 127, 127, 0.08));
  font-size: 12px;
  line-height: 1.5;
}
.klog-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-3) 0 0;
}
.klog-footer__hint {
  margin-left: auto;
}

@media (max-width: 768px) {
  .klog-row__main {
    grid-template-columns: 1fr 48px 16px;
  }
  .klog-row__time,
  .klog-row__target {
    display: none;
  }
  .klog-row__msg {
    grid-column: 1;
    grid-row: 1;
  }
  .klog-detail {
    padding-left: var(--fp-space-3);
  }
  .klog-footer__hint {
    display: none;
  }
}

@keyframes klog-pulse {
  50% {
    opacity: 0.35;
  }
}
@keyframes klog-fresh {
  from {
    background-color: color-mix(in srgb, var(--fp-brand) 18%, transparent);
  }
}
@keyframes klog-open {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .klog-live__dot,
  .klog-row.is-fresh,
  .klog-detail {
    animation: none;
  }
  .klog-level,
  .klog-row__chevron,
  .klog-list {
    transition: none;
  }
}
</style>
