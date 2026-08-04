<template>
  <div class="fp-table">
    <DataTable
      v-bind="$attrs"
      :value="rows"
      :size="size"
      :loading="loading"
      :striped-rows="stripedRows"
      :row-hover="true"
      :scrollable="virtual"
      :virtual-scroller-options="virtual ? virtualOptions : undefined"      :paginator="paginator"
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
    /** 虚拟滚动（大列表） */
    virtual?: boolean
    virtualItemSize?: number
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

const virtualOptions = {
  itemSize: 44,
  scrollHeight: '520px',
}

function emitFirst(value: number) {
  emit('update:first', value)
}
</script>

<style scoped>
.fp-table {
  width: 100%;
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
