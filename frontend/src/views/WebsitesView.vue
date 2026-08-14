<template>
  <div class="view-container">
    <div class="page-toolbar">
      <div class="toolbar-left">
        <FpInput
          v-model="searchText"
          :placeholder="t('common.searchPlaceholder')"
          class="toolbar-search"
        />
        <FpSelect
          v-model="statusFilter"
          :options="statusOptions"
          option-label="label"
          option-value="value"
          show-clear
          class="toolbar-filter"
        />
      </div>
      <FpButton v-permission="{ perm: 'website:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('website.create') }}
      </FpButton>
    </div>

    <div class="panel">
      <FpStatePanel
        :loading="loading"
        :error="websitesError"
        :empty="!total && !loading && !websitesError"
        retryable
        :empty-title="t('common.noData')"
        @retry="fetch"
      >
        <FpTable
          :rows="filteredWebsites"
          :loading="loading"
          :empty-text="t('common.noData')"
          :first="(currentPage - 1) * pageSize"
          striped-rows
        >
          <FpColumn field="id" :header="t('website.id')" style="width: 60px" />
          <FpColumn field="name" :header="t('website.name')" />
          <FpColumn field="domain" :header="t('website.domain')" />
          <FpColumn field="root_path" :header="t('website.rootPath')" />
          <FpColumn field="node_id" :header="t('website.nodeId')" style="width: 80px" />
          <FpColumn :header="t('website.status')" style="width: 100px">
            <template #body="{ data }">
              <FpTag :severity="data.status === 'active' ? 'success' : 'info'">
                {{ data.status === 'active' ? t('website.active') : t('website.inactive') }}
              </FpTag>
            </template>
          </FpColumn>
          <FpColumn field="created_at" :header="t('website.createdAt')" style="width: 180px" />
          <FpColumn :header="t('common.operation')" style="width: 260px" frozen>
            <template #body="{ data }">
              <div class="row-actions">
                <FpButton v-permission="{ perm: 'website:update', mode: 'view' }" variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
                <FpButton v-permission="{ perm: 'website:update', mode: 'view' }" variant="link" @click="openSwitchEngine(data)">{{
                  t('website.switchEngine')
                }}</FpButton>
                <FpButton v-permission="{ perm: 'website:delete', mode: 'view' }" variant="link" @click="confirmDelete(data)">{{ t('common.delete') }}</FpButton>
              </div>
            </template>
          </FpColumn>
        </FpTable>
        <FpPagination
          v-if="total > pageSize"
          :first="(currentPage - 1) * pageSize"
          :rows="pageSize"
          :total="total"
          :rows-per-page-options="[20, 50, 100]"
          @update:first="(f) => goPage(f)"
        />
      </FpStatePanel>
    </div>

    <FpModal v-model="showCreate" :header="t('website.create')" style="width: 500px">
      <div class="modal-form">
        <FpInput v-model="form.name" :label="t('website.name')" :error="formErrors.name" />
        <FpInput v-model="form.domain" :label="t('website.domain')" :error="formErrors.domain" />
        <FpInput v-model="form.root_path" :label="t('website.rootPath')" />
        <div class="field-col">
          <label class="field-label">{{ t('website.nodeId') }}</label>
          <FpNumber v-model="form.node_id" :min="1" class="w-full" />
        </div>
        <FpSelect
          v-model="form.engine"
          :label="t('website.engine')"
          :options="engineOptions"
          option-label="label"
          option-value="value"
        />
        <div class="field-col field-row">
          <label class="field-label">{{ t('website.sslEnabled') }}</label>
          <FpSwitch v-model="form.ssl_enabled" />
        </div>
        <div class="field-col field-row">
          <label class="field-label">{{ t('website.proxyEnabled') }}</label>
          <FpSwitch v-model="form.proxy_enabled" />
        </div>
        <FpInput v-if="form.proxy_enabled" v-model="form.proxy_pass" :label="t('website.proxyPass')" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="showCreate = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleCreate">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="editVisible" :header="t('website.edit')" style="width: 500px">
      <div class="modal-form">
        <FpInput v-model="editForm.name" :label="t('website.name')" :error="formErrors.name" />
        <FpInput v-model="editForm.domain" :label="t('website.domain')" :error="formErrors.domain" />
        <FpInput v-model="editForm.root_path" :label="t('website.rootPath')" />
        <div class="field-col">
          <label class="field-label">{{ t('website.nodeId') }}</label>
          <FpNumber v-model="editForm.node_id" :min="1" class="w-full" />
        </div>
        <FpSelect
          v-model="editForm.engine"
          :label="t('website.engine')"
          :options="engineOptions"
          option-label="label"
          option-value="value"
        />
        <div class="field-col field-row">
          <label class="field-label">{{ t('website.sslEnabled') }}</label>
          <FpSwitch v-model="editForm.ssl_enabled" />
        </div>
        <div class="field-col field-row">
          <label class="field-label">{{ t('website.proxyEnabled') }}</label>
          <FpSwitch v-model="editForm.proxy_enabled" />
        </div>
        <FpInput
          v-if="editForm.proxy_enabled"
          v-model="editForm.proxy_pass"
          :label="t('website.proxyPass')"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="editVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSave">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <FpModal v-model="switchVisible" :header="t('website.switchEngine')" style="width: 440px">
      <div class="modal-form">
        <FpSelect
          v-model="switchEngine"
          :label="t('website.engine')"
          :options="engines"
          option-label="name"
          option-value="name"
        />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="switchVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSwitchEngine">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'




import { listWebsites, createWebsite, updateWebsite, deleteWebsite, switchWebsiteEngine } from '@/api/websites'
import { listEngines } from '@/api/webServers'
import type { Website, EngineInfo } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type { Page } from '@/api/generated'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const currentPage = ref(1)
const pageSize = ref(20)
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
const formErrors = reactive({ name: '', domain: '' })

const engineOptions = [
  { label: 'nginx', value: 'nginx' },
  { label: 'apache', value: 'apache' },
  { label: 'openlitespeed', value: 'openlitespeed' },
  { label: 'openresty', value: 'openresty' },
  { label: 'caddy', value: 'caddy' },
]

// Modernization M2：分页走统一数据获取层，切换页保留上一页数据（keepPreviousData）
const websitesQuery = useApiQuery<Page<Website>>(
  () => queryKeys.websites.list(currentPage.value, pageSize.value),
  async () => {
    const res = await listWebsites(currentPage.value, pageSize.value)
    return { data: res.data }
  },
  { keepPrevious: true },
)
const websites = computed<Page<Website>['data']>(() => websitesQuery.data.value?.data ?? [])
const total = computed(() => websitesQuery.data.value?.total ?? 0)
const loading = websitesQuery.loading
const websitesError = websitesQuery.error

// M9：搜索 + 状态筛选（当前页客户端过滤）
const searchText = ref('')
const statusFilter = ref<string>('')
const statusOptions = computed(() => [
  { label: t('website.active'), value: 'active' },
  { label: t('website.inactive'), value: 'inactive' },
])
const filteredWebsites = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return websites.value.filter((w) => {
    if (statusFilter.value && w.status !== statusFilter.value) return false
    if (!kw) return true
    return (
      w.name.toLowerCase().includes(kw) ||
      w.domain.toLowerCase().includes(kw) ||
      w.root_path.toLowerCase().includes(kw)
    )
  })
})

async function fetch() {
  await websitesQuery.refresh()
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  void fetch()
}

function openCreate() {
  form.name = ''
  form.domain = ''
  form.root_path = ''
  form.node_id = 1
  form.engine = 'nginx'
  form.ssl_enabled = false
  form.proxy_enabled = false
  form.proxy_pass = ''
  formErrors.name = ''
  formErrors.domain = ''
  showCreate.value = true
}

function validateForm(): boolean {
  formErrors.name = form.name ? '' : t('common.required')
  formErrors.domain = form.domain ? '' : t('common.required')
  return !formErrors.name && !formErrors.domain
}

async function handleCreate() {
  if (!validateForm()) return
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
    toast.success(t('common.success'))
    showCreate.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
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
  formErrors.name = ''
  formErrors.domain = ''
  editVisible.value = true
}

async function handleSave() {
  if (!editForm.name || !editForm.domain) {
    formErrors.name = editForm.name ? '' : t('common.required')
    formErrors.domain = editForm.domain ? '' : t('common.required')
    return
  }
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
    toast.success(t('common.success'))
    editVisible.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function confirmDelete(row: Website) {
  confirmAction({
    message: t('website.deleteConfirm', { name: row.name }),
    header: t('common.confirmAction'),
    accept: async () => {
      try {
        await deleteWebsite(row.id)
        toast.success(t('common.success'))
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
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
      toast.error(t('common.failed'))
    }
  }
  switchVisible.value = true
}

async function handleSwitchEngine() {
  if (!switchEngine.value) return
  submitting.value = true
  try {
    await switchWebsiteEngine(switchTargetId, switchEngine.value)
    toast.success(t('common.success'))
    switchVisible.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.page-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.toolbar-search {
  width: 240px;
}
.toolbar-filter {
  width: 160px;
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.field-col {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-row {
  flex-direction: row;
  align-items: center;
  gap: var(--fp-space-3);
}
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
</style>
