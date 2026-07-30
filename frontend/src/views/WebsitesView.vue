<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.websites') }}</h2>
      <el-button type="primary" @click="showCreate = true">{{ t('website.create') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table :data="websites" border stripe v-loading="loading">
        <el-table-column prop="id" :label="t('website.id')" width="60" />
        <el-table-column prop="name" :label="t('website.name')" />
        <el-table-column prop="domain" :label="t('website.domain')" />
        <el-table-column prop="root_path" :label="t('website.rootPath')" />
        <el-table-column prop="node_id" :label="t('website.nodeId')" width="80" />
        <el-table-column :label="t('website.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? t('website.active') : t('website.inactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('website.createdAt')" width="180" />
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        background
        small
        style="margin-top: 16px; justify-content: center;"
        @current-change="fetch"
      />
    </el-card>

    <el-dialog v-model="showCreate" :title="t('website.create')" width="500px">
      <el-form :model="form" label-width="100px">
        <el-form-item :label="t('website.name')">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('website.domain')">
          <el-input v-model="form.domain" />
        </el-form-item>
        <el-form-item :label="t('website.rootPath')">
          <el-input v-model="form.root_path" />
        </el-form-item>
        <el-form-item :label="t('website.nodeId')">
          <el-input-number v-model="form.node_id" :min="1" style="width:100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="handleCreate" :loading="submitting">{{ t('common.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listWebsites, createWebsite } from '@/api/websites'
import { ElMessage } from 'element-plus'
import type { Website } from '@/types'

const { t } = useI18n()
const websites = ref<Website[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const showCreate = ref(false)
const submitting = ref(false)
const form = reactive({ name: '', domain: '', root_path: '', node_id: 1, status: 'active' as string })

async function fetch() {
  loading.value = true
  try { const res = await listWebsites(currentPage.value, pageSize.value); websites.value = res.data.data; total.value = res.data.total }
  finally { loading.value = false }
}

async function handleCreate() {
  submitting.value = true
  try {
    await createWebsite({ id: 0, name: form.name, domain: form.domain, root_path: form.root_path, node_id: form.node_id, status: 'active', created_at: '' })
    ElMessage.success(t('common.success'))
    showCreate.value = false
    await fetch()
  } catch { ElMessage.error(t('common.failed')) }
  finally { submitting.value = false }
}

onMounted(fetch)
</script>
