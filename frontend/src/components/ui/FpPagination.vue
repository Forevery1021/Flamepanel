<template>
  <Paginator
    v-if="total > pageSize"
    :first="first"
    :rows="pageSize"
    :total-records="total"
    :rows-per-page-options="rowsPerPageOptions"
    :template="template"
    :current-page-report-template="currentPageReportTemplate"
    @update:first="(f) => emit('update:first', f)"
  />
</template>

<script setup lang="ts">
import Paginator from 'openvue/paginator'

withDefaults(
  defineProps<{
    total: number
    pageSize?: number
    first?: number
    rowsPerPageOptions?: number[]
    template?: string
    currentPageReportTemplate?: string
  }>(),
  {
    pageSize: 20,
    first: 0,
    rowsPerPageOptions: () => [20, 50, 100],
    template: 'FirstPageLink PrevPageLink PageLinks NextPageLink LastPageLink RowsPerPageDropdown',
    currentPageReportTemplate: '{first}-{last} / {totalRecords}',
  },
)

const emit = defineEmits<{ 'update:first': [value: number] }>()
</script>
