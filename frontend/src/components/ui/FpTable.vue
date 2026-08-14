<template>
  <div class="fp-table">
    <DataTable
      v-bind="$attrs"
      :value="rows"
      :size="size"
      :loading="loading"
      :striped-rows="stripedRows"
      :row-hover="true"
      :sort-mode="sortable ? 'single' : undefined"
      :sort-field="sortField || undefined"
      :sort-order="sortOrder"
      :scrollable="virtual || scrollable"
      :scroll-height="virtualScrollHeight || scrollHeight || undefined"
      :virtual-scroller-options="
        virtual
          ? {
              itemSize: virtualItemSize,
              scrollHeight: virtualScrollHeight || '520px',
            }
          : undefined
      "
      :paginator="paginator && !virtual"
      :rows="rowsPerPage"
      :first="first"
      :rows-per-page-options="rowsPerPageOptions"
      :paginator-template="paginatorTemplate"
      :current-page-report-template="currentPageReportTemplate"
      :empty-message="emptyText"
      @update:first="emitFirst"
    >
      <template #empty>
        <FpEmpty :title="emptyText" :description="emptyDesc" />
      </template>
      <template #loading>
        <div class="fp-table__loading">
          <i class="oi oi-spinner fp-spin" />
          <span>{{ loadingText }}</span>
        </div>
      </template>
      <slot />
    </DataTable>
  </div>
</template>

<script setup lang="ts">
import DataTable from 'openvue/datatable'
import FpEmpty from './FpEmpty.vue'

withDefaults(
  defineProps<{
    /** 表格数据 */
    rows: unknown[]
    loading?: boolean
    stripedRows?: boolean
    /** 虚拟滚动（大列表；启用后自动 scrollable，自动禁用分页） */
    virtual?: boolean
    virtualItemSize?: number
    /** 虚拟滚动视口高度（默认 520px） */
    virtualScrollHeight?: string
    /** 普通滚动模式（无虚拟化，配合 scrollHeight） */
    scrollable?: boolean
    scrollHeight?: string
    /** 客户端单列排序（`sortable` 开启时生效） */
    sortable?: boolean
    sortField?: string | null
    sortOrder?: 1 | -1 | 0
    paginator?: boolean
    rowsPerPage?: number
    first?: number
    rowsPerPageOptions?: number[]
    size?: 'small' | 'large'
    paginatorTemplate?: string
    currentPageReportTemplate?: string
    emptyText?: string
    emptyDesc?: string
    loadingText?: string
  }>(),
  {
    loading: false,
    stripedRows: false,
    virtual: false,
    virtualItemSize: 44,
    virtualScrollHeight: '520px',
    scrollable: false,
    scrollHeight: '',
    sortable: false,
    sortField: null,
    sortOrder: 0,
    paginator: true,
    rowsPerPage: 10,
    first: 0,
    rowsPerPageOptions: () => [10, 20, 50, 100],
    size: 'small',
    paginatorTemplate:
      'FirstPageLink PrevPageLink PageLinks NextPageLink LastPageLink RowsPerPageDropdown',
    currentPageReportTemplate: '{first}-{last} / {totalRecords}',
    emptyText: '',
    emptyDesc: '',
    loadingText: '',
  },
)

const emit = defineEmits<{ 'update:first': [value: number] }>()

function emitFirst(value: number) {
  emit('update:first', value)
}
</script>

<style scoped>
.fp-table {
  width: 100%;
  /* 窄屏下表格可横向滚动（F3.3 响应式底线） */
  overflow-x: auto;
}
.fp-table :deep(.p-datatable-wrapper) {
  overflow-x: auto;
}
.fp-table__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-6);
  color: var(--fp-text-secondary);
  font-size: 13px;
}
.fp-spin {
  animation: fp-rotate 0.8s linear infinite;
}
@keyframes fp-rotate {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .fp-spin {
    animation: none;
  }
}
</style>
