<template>
  <div class="view-container">
    <div class="card-header-title">
      <el-button type="primary" @click="showCreate = true">{{ t('website.create') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :empty-text="t('common.noData')" :data="websites" border stripe>
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
        <el-table-column :label="t('common.operation')" width="260" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" size="small" text @click="handleEdit(row)">{{
              t('common.edit')
            }}</el-button>
            <el-button type="primary" size="small" text @click="openSwitchEngine(row)">{{
              t('website.switchEngine')
            }}</el-button>
            <el-popconfirm
              :title="t('website.deleteConfirm', { name: row.name })"
              @confirm="handleDelete(row.id)"
            >
              <template #reference>
                <el-button type="danger" size="small" text>{{ t('common.delete') }}</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
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
          <el-input-number v-model="form.node_id" :min="1" class="full-width" />
        </el-form-item>
        <el-form-item :label="t('website.engine')">
          <el-select v-model="form.engine" class="full-width">
            <el-option label="nginx" value="nginx" />
            <el-option label="apache" value="apache" />
            <el-option label="openlitespeed" value="openlitespeed" />
            <el-option label="openresty" value="openresty" />
            <el-option label="caddy" value="caddy" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('website.sslEnabled')">
          <el-switch v-model="form.ssl_enabled" />
        </el-form-item>
        <el-form-item :label="t('website.proxyEnabled')">
          <el-switch v-model="form.proxy_enabled" />
        </el-form-item>
        <el-form-item v-if="form.proxy_enabled" :label="t('website.proxyPass')">
          <el-input v-model="form.proxy_pass" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreate">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="editVisible" :title="t('website.edit')" width="500px">
      <el-form :model="editForm" label-width="100px">
        <el-form-item :label="t('website.name')">
          <el-input v-model="editForm.name" />
        </el-form-item>
        <el-form-item :label="t('website.domain')">
          <el-input v-model="editForm.domain" />
        </el-form-item>
        <el-form-item :label="t('website.rootPath')">
          <el-input v-model="editForm.root_path" />
        </el-form-item>
        <el-form-item :label="t('website.nodeId')">
          <el-input-number v-model="editForm.node_id" :min="1" class="full-width" />
        </el-form-item>
        <el-form-item :label="t('website.engine')">
          <el-select v-model="editForm.engine" class="full-width">
            <el-option label="nginx" value="nginx" />
            <el-option label="apache" value="apache" />
            <el-option label="openlitespeed" value="openlitespeed" />
            <el-option label="openresty" value="openresty" />
            <el-option label="caddy" value="caddy" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('website.sslEnabled')">
          <el-switch v-model="editForm.ssl_enabled" />
        </el-form-item>
        <el-form-item :label="t('website.proxyEnabled')">
          <el-switch v-model="editForm.proxy_enabled" />
        </el-form-item>
        <el-form-item v-if="editForm.proxy_enabled" :label="t('website.proxyPass')">
          <el-input v-model="editForm.proxy_pass" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSave">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="switchVisible" :title="t('website.switchEngine')" width="440px">
      <el-form label-width="100px">
        <el-form-item :label="t('website.engine')" required>
          <el-select v-model="switchEngine" class="full-width">
            <el-option v-for="e in engines" :key="e.name" :label="e.name" :value="e.name" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="switchVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSwitchEngine">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listWebsites, createWebsite, updateWebsite, deleteWebsite, switchWebsiteEngine } from '@/api/websites'
import { listEngines } from '@/api/webServers'
import { ElMessage } from 'element-plus'
import type { Website, EngineInfo } from '@/types'

const { t } = useI18n()
const websites = ref<Website[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const showCreate = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)
const form = reactive({
  name: '',
  domain: '',
  root_path: '',
  node_id: 1,
  status: 'active' as string,
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: '',
})
const editForm = reactive({
  name: '',
  domain: '',
  root_path: '',
  node_id: 1,
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: '',
})

async function fetch() {
  loading.value = true
  try {
    const res = await listWebsites(currentPage.value, pageSize.value)
    websites.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  submitting.value = true
  try {
    await createWebsite({
      id: 0,
      name: form.name,
      domain: form.domain,
      root_path: form.root_path,
      node_id: form.node_id,
      status: 'active',
      engine: form.engine,
      ssl_enabled: form.ssl_enabled,
      proxy_enabled: form.proxy_enabled,
      proxy_pass: form.proxy_enabled ? form.proxy_pass : null,
      created_at: '',
    })
    ElMessage.success(t('common.success'))
    showCreate.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function handleEdit(row: Website) {
  editingId.value = row.id
  editForm.name = row.name
  editForm.domain = row.domain
  editForm.root_path = row.root_path
  editForm.node_id = row.node_id
  editForm.engine = row.engine || 'nginx'
  editForm.ssl_enabled = row.ssl_enabled
  editForm.proxy_enabled = row.proxy_enabled
  editForm.proxy_pass = row.proxy_pass || ''
  editVisible.value = true
}

async function handleSave() {
  submitting.value = true
  try {
    await updateWebsite(editingId.value, {
      id: editingId.value,
      name: editForm.name,
      domain: editForm.domain,
      root_path: editForm.root_path,
      node_id: editForm.node_id,
      status: 'active',
      engine: editForm.engine,
      ssl_enabled: editForm.ssl_enabled,
      proxy_enabled: editForm.proxy_enabled,
      proxy_pass: editForm.proxy_enabled ? editForm.proxy_pass : null,
      created_at: '',
    })
    ElMessage.success(t('common.success'))
    editVisible.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(id: number) {
  try {
    await deleteWebsite(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

const switchVisible = ref(false)
const switchEngine = ref('')
const engines = ref<EngineInfo[]>([])
let switchTargetId = 0

async function openSwitchEngine(row: Website) {
  switchTargetId = row.id
  switchEngine.value = row.engine
  if (!engines.value.length) {
    try {
      const res = await listEngines()
      engines.value = res.data
    } catch {
      ElMessage.error(t('common.failed'))
    }
  }
  switchVisible.value = true
}

async function handleSwitchEngine() {
  if (!switchEngine.value) return
  submitting.value = true
  try {
    await switchWebsiteEngine(switchTargetId, switchEngine.value)
    ElMessage.success(t('common.success'))
    switchVisible.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

onMounted(fetch)
</script>
