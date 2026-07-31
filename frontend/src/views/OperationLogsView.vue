<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.operationLogs') }}</h2>
      <el-button text @click="fetch">{{ t('common.refresh') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :empty-text="t('common.noData')"
        :data="logs"
        border
        stripe
        max-height="620px"
      >
        <el-table-column prop="id" :label="t('log.id')" width="60" />
        <el-table-column prop="username" :label="t('log.user')" width="120" />
        <el-table-column :label="t('log.action')" width="140">
          <template #default="{ row }">
            <el-tag size="small">{{ row.action }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="target" :label="t('log.target')" min-width="200" />
        <el-table-column prop="ip" :label="t('log.ip')" width="140" />
        <el-table-column prop="created_at" :label="t('log.time')" width="180" />
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        background
        small
        class="table-pagination"
        @current-change="fetch"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listOperationLogs } from '@/api/logs'
import type { OperationLog } from '@/types'

const { t } = useI18n()
const logs = ref<OperationLog[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)

async function fetch() {
  loading.value = true
  try {
    const res = await listOperationLogs(currentPage.value, pageSize.value)
    logs.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

onMounted(fetch)
</script>
