<template>
  <article :id="anchorId" class="api-op" :class="[`is-${op.method}`, { 'is-open': open }]">
    <button
      type="button"
      class="api-op__head"
      :aria-expanded="open"
      :aria-controls="`${anchorId}-body`"
      @click="emit('toggle')"
    >
      <span class="api-op__method" :class="`m-${op.method}`">{{ op.method.toUpperCase() }}</span>
      <code class="api-op__path">{{ op.path }}</code>
      <span class="api-op__summary">{{ op.summary }}</span>
      <span class="api-op__meta">
        <i
          :class="op.secured ? 'oi oi-lock' : 'oi oi-lock-open'"
          :title="op.secured ? t('apiDocs.authBearer') : t('apiDocs.authNone')"
          :aria-label="op.secured ? t('apiDocs.authBearer') : t('apiDocs.authNone')"
        />
        <FpTag v-if="op.permission" :value="op.permission" severity="info" />
        <i class="oi oi-chevron-down api-op__chevron" aria-hidden="true" />
      </span>
    </button>

    <div :id="`${anchorId}-body`" class="api-op__body-wrap" :aria-hidden="!open">
      <div class="api-op__body">
        <template v-if="open">
          <p v-if="op.description && op.description !== op.summary" class="api-op__desc">
            {{ op.description }}
          </p>

          <dl class="api-op__facts">
            <div>
              <dt>{{ t('apiDocs.auth') }}</dt>
              <dd>{{ op.secured ? t('apiDocs.authBearer') : t('apiDocs.authNone') }}</dd>
            </div>
            <div>
              <dt>{{ t('apiDocs.permission') }}</dt>
              <dd>
                <code v-if="op.permission">{{ op.permission }}</code>
                <span v-else>{{
                  op.secured ? t('apiDocs.permissionLogin') : t('apiDocs.permissionPublic')
                }}</span>
              </dd>
            </div>
            <div v-if="op.operationId">
              <dt>operationId</dt>
              <dd>
                <code>{{ op.operationId }}</code>
              </dd>
            </div>
          </dl>

          <section v-if="op.parameters.length" class="api-op__section">
            <h4>{{ t('apiDocs.parameters') }}</h4>
            <table class="api-op__table">
              <thead>
                <tr>
                  <th>{{ t('apiDocs.name') }}</th>
                  <th>{{ t('apiDocs.location') }}</th>
                  <th>{{ t('apiDocs.type') }}</th>
                  <th>{{ t('apiDocs.required') }}</th>
                  <th>{{ t('apiDocs.descriptionCol') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="p in op.parameters" :key="`${p.in}:${p.name}`">
                  <td>
                    <code>{{ p.name }}</code>
                  </td>
                  <td>{{ p.in }}</td>
                  <td>
                    <code>{{ typeName(p.schema) }}</code>
                  </td>
                  <td>{{ p.required ? t('apiDocs.yes') : t('apiDocs.no') }}</td>
                  <td>{{ p.description }}</td>
                </tr>
              </tbody>
            </table>
          </section>

          <section v-if="op.requestBody" class="api-op__section">
            <h4>
              {{ t('apiDocs.requestBody') }}
              <code class="api-op__ctype">{{ op.requestBody.contentType }}</code>
              <code>{{ typeName(op.requestBody.schema) }}</code>
            </h4>
            <table v-if="bodyFields.length" class="api-op__table">
              <thead>
                <tr>
                  <th>{{ t('apiDocs.field') }}</th>
                  <th>{{ t('apiDocs.type') }}</th>
                  <th>{{ t('apiDocs.required') }}</th>
                  <th>{{ t('apiDocs.descriptionCol') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="f in bodyFields" :key="f.name">
                  <td>
                    <code>{{ f.name }}</code>
                  </td>
                  <td>
                    <code>{{ f.type }}</code>
                  </td>
                  <td>{{ f.required ? t('apiDocs.yes') : t('apiDocs.no') }}</td>
                  <td>{{ f.description }}</td>
                </tr>
              </tbody>
            </table>
            <div v-if="bodyJson" class="api-op__code">
              <pre>{{ bodyJson }}</pre>
              <CopyButton :text="bodyJson" />
            </div>
          </section>

          <section class="api-op__section">
            <h4>{{ t('apiDocs.responses') }}</h4>
            <table class="api-op__table">
              <tbody>
                <tr v-for="r in op.responses" :key="r.status">
                  <td class="api-op__status" :class="`s-${r.status[0]}`">{{ r.status }}</td>
                  <td>{{ r.description }}</td>
                  <td>
                    <code v-if="r.schema">{{ typeName(r.schema) }}</code>
                    <code v-else-if="r.contentType">{{ r.contentType }}</code>
                  </td>
                </tr>
              </tbody>
            </table>
          </section>

          <section class="api-op__section">
            <h4>{{ t('apiDocs.tryIt') }}</h4>
            <p class="api-op__hint">{{ t('apiDocs.fpctlHint') }}</p>
            <div class="api-op__code">
              <pre>{{ fpctl }}</pre>
              <CopyButton :text="fpctl" />
            </div>
            <div class="api-op__code">
              <pre>{{ curl }}</pre>
              <CopyButton :text="curl" />
            </div>
          </section>
        </template>
      </div>
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import FpTag from '@/components/ui/FpTag.vue'
import CopyButton from '@/components/ui/CopyButton.vue'
import {
  curlCommand,
  fpctlCommand,
  requestExample,
  schemaFields,
  typeName,
  type ApiOperation,
  type OpenApiDocument,
} from '@/utils/openapi'

const props = defineProps<{
  op: ApiOperation
  doc: OpenApiDocument
  open: boolean
  /** 页面来源（curl 示例的主机部分） */
  origin: string
}>()
const emit = defineEmits<{ toggle: [] }>()
const { t } = useI18n()

/** 可作 URL 片段的锚点：`op-get-api-users-id` */
const anchorId = computed(
  () =>
    `op-${props.op.method}-${props.op.path.replace(/[^a-zA-Z0-9]+/g, '-').replace(/^-|-$/g, '')}`,
)

const body = computed(() => (props.open ? requestExample(props.doc, props.op) : undefined))
const bodyJson = computed(() =>
  body.value === undefined ? '' : JSON.stringify(body.value, null, 2),
)
const bodyFields = computed(() =>
  props.open && props.op.requestBody ? schemaFields(props.doc, props.op.requestBody.schema) : [],
)
const fpctl = computed(() => fpctlCommand(props.op, body.value))
const curl = computed(() => curlCommand(props.op, props.origin, body.value))
</script>

<style scoped>
.api-op {
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-surface);
  transition:
    border-color var(--fp-transition-fast),
    box-shadow var(--fp-transition-fast);
}
.api-op:hover,
.api-op.is-open {
  border-color: var(--fp-border-strong);
}
.api-op.is-open {
  box-shadow: var(--fp-shadow-sm);
}
.api-op__head {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  width: 100%;
  padding: var(--fp-space-2) var(--fp-space-3);
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  border-radius: var(--fp-radius-md);
}
.api-op__head:focus-visible {
  outline: none;
  box-shadow: var(--fp-focus-ring);
}
.api-op__method {
  flex: none;
  min-width: 64px;
  padding: 2px 8px;
  border-radius: var(--fp-radius-sm);
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-xs);
  font-weight: 700;
  text-align: center;
  letter-spacing: 0.02em;
}
.m-get {
  background: var(--fp-info-soft);
  color: var(--fp-info);
}
.m-post {
  background: var(--fp-success-soft);
  color: var(--fp-success);
}
.m-put,
.m-patch {
  background: var(--fp-warning-soft);
  color: var(--fp-warning);
}
.m-delete {
  background: var(--fp-danger-soft);
  color: var(--fp-danger);
}
.api-op__path {
  flex: none;
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-sm);
  color: var(--fp-text-primary);
  word-break: break-all;
}
.api-op__summary {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-sm);
  white-space: nowrap;
  text-overflow: ellipsis;
}
.api-op__meta {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--fp-space-2);
  color: var(--fp-text-muted);
}
.api-op__chevron {
  transition: transform var(--fp-transition-base);
}
.api-op.is-open .api-op__chevron {
  transform: rotate(180deg);
}
/* 展开动画：grid 行从 0fr 过渡到 1fr，高度自适应且无需测量 */
.api-op__body-wrap {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows var(--fp-transition-base);
}
.api-op.is-open .api-op__body-wrap {
  grid-template-rows: 1fr;
}
.api-op__body {
  overflow: hidden;
  min-height: 0;
}
.api-op.is-open .api-op__body {
  padding: 0 var(--fp-space-4) var(--fp-space-4);
  border-top: 1px dashed var(--fp-border);
}
.api-op__desc {
  margin: var(--fp-space-3) 0 0;
  color: var(--fp-text-secondary);
  white-space: pre-line;
}
.api-op__facts {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2) var(--fp-space-6);
  margin: var(--fp-space-3) 0 0;
  font-size: var(--fp-font-sm);
}
.api-op__facts dt {
  color: var(--fp-text-muted);
}
.api-op__facts dd {
  margin: 0;
}
.api-op__section h4 {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
  margin: var(--fp-space-4) 0 var(--fp-space-2);
  font-size: var(--fp-font-sm);
  color: var(--fp-text-primary);
}
.api-op__ctype {
  color: var(--fp-text-muted);
}
.api-op__table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fp-font-sm);
}
.api-op__table th,
.api-op__table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--fp-border);
  text-align: left;
  vertical-align: top;
}
.api-op__table th {
  color: var(--fp-text-muted);
  font-weight: 500;
}
.api-op__status {
  width: 56px;
  font-family: var(--fp-font-mono);
  font-weight: 600;
}
.s-2 {
  color: var(--fp-success);
}
.s-4 {
  color: var(--fp-warning);
}
.s-5 {
  color: var(--fp-danger);
}
.api-op code {
  font-family: var(--fp-font-mono);
  color: var(--fp-text-code);
}
.api-op__hint {
  margin: 0 0 var(--fp-space-2);
  color: var(--fp-text-muted);
  font-size: var(--fp-font-xs);
}
.api-op__code {
  position: relative;
  margin-bottom: var(--fp-space-2);
}
.api-op__code pre {
  margin: 0;
  padding: var(--fp-space-3);
  padding-right: 44px;
  overflow-x: auto;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-subtle);
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-xs);
  line-height: 1.6;
  white-space: pre;
}
.api-op__code :deep(.p-button) {
  position: absolute;
  top: 4px;
  right: 4px;
}
@media (prefers-reduced-motion: reduce) {
  .api-op__body-wrap,
  .api-op__chevron {
    transition: none;
  }
}
@media (max-width: 720px) {
  .api-op__head {
    flex-wrap: wrap;
  }
  .api-op__summary {
    flex-basis: 100%;
    white-space: normal;
  }
}
</style>
