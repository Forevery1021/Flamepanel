<template>
  <LayoutContent :title="t('user.title')" reload @reload="fetch">
    <template #toolbar>
      <div class="toolbar-left">
        <FpInput
          v-model="searchText"
          :placeholder="t('common.searchPlaceholder')"
          class="toolbar-search"
        />
        <FpSelect
          v-model="roleFilter"
          :options="roleOptions"
          option-label="label"
          option-value="value"
          show-clear
          class="toolbar-filter"
        />
      </div>
      <FpButton v-permission="{ perm: 'user:create', mode: 'view' }" variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('user.createUser') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpStatePanel
        :loading="loading"
        :error="usersError"
        :empty="!total && !loading && !usersError"
        retryable
        :empty-title="t('common.noData')"
        @retry="fetch"
      >
        <FpTable :rows="filteredUsers" :loading="loading" :first="(currentPage - 1) * pageSize">
          <FpColumn field="id" :header="t('user.id')" style="width: 80px" />
          <FpColumn field="username" :header="t('user.username')" />
          <FpColumn :header="t('user.role')" style="width: 140px">
            <template #body="{ data }">
              <FpTag :severity="roleSeverity(data.role)" :value="roleLabel(data.role)" />
            </template>
          </FpColumn>
          <FpColumn field="created_at" :header="t('user.createdAt')" style="width: 190px">
            <template #body="{ data }">
              <span class="mono">{{ data.created_at }}</span>
            </template>
          </FpColumn>
          <FpColumn :header="t('common.operation')" style="width: 160px" frozen>
            <template #body="{ data }">
              <div class="row-actions">
                <FpButton v-permission="{ perm: 'user:update', mode: 'view' }" variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
                <FpButton v-permission="{ perm: 'user:delete', mode: 'view' }" variant="link" @click="confirmDelete(data)">{{ t('common.delete') }}</FpButton>
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

    <!-- 创建 -->
    <FpModal v-model="dialogVisible" :header="t('user.createUser')">
      <div class="modal-form">
        <FpInput v-model="form.username" :label="t('user.username')" :error="formErrors.username" />
        <FpInput v-model="form.password" :label="t('user.password')" type="password" toggle-mask :error="formErrors.password" />
        <FpSelect v-model="form.role" :label="t('user.role')" :options="roleOptions" option-label="label" option-value="value" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleCreate">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>

    <!-- 编辑 -->
    <FpModal v-model="editVisible" :header="t('user.editUser')">
      <div class="modal-form">
        <FpInput v-model="editForm.username" :label="t('user.username')" :error="formErrors.username" />
        <FpInput v-model="editForm.password" :label="t('user.password')" type="password" toggle-mask :placeholder="t('user.passwordOptional')" />
        <FpSelect v-model="editForm.role" :label="t('user.role')" :options="roleOptions" option-label="label" option-value="value" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="editVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="submitting" @click="handleSave">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>
  </LayoutContent>
</template>

<script setup lang="ts">
import { reactive, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'


import { listUsers, createUser, updateUser, deleteUser } from '@/api/users'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import type { User, Page } from '@/api/generated'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const currentPage = ref(1)
const pageSize = ref(20)
const dialogVisible = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const editingId = ref(0)

const form = reactive({ username: '', password: '', role: 'viewer' })
const editForm = reactive({ username: '', password: '', role: 'viewer' })
const formErrors = reactive({ username: '', password: '' })

const roleOptions = computed(() => [
  { label: t('user.admin'), value: 'admin' },
  { label: t('user.operator'), value: 'operator' },
  { label: t('user.viewer'), value: 'viewer' },
])

function roleLabel(role: string) {
  const map: Record<string, string> = {
    admin: t('user.admin'),
    operator: t('user.operator'),
    viewer: t('user.viewer'),
  }
  return map[role] || role
}

function roleSeverity(role: string) {
  return role === 'admin' ? 'danger' : role === 'operator' ? 'warning' : 'info'
}

// Modernization M2：分页走统一数据获取层，切换页保留上一页数据（keepPreviousData）
const usersQuery = useApiQuery<Page<User>>(
  () => queryKeys.users.list(currentPage.value, pageSize.value),
  async () => {
    const res = await listUsers(currentPage.value, pageSize.value)
    return { data: res.data }
  },
  { keepPrevious: true },
)
const users = computed<Page<User>['data']>(() => usersQuery.data.value?.data ?? [])
const total = computed(() => usersQuery.data.value?.total ?? 0)
const loading = usersQuery.loading
const usersError = usersQuery.error

// M9：搜索 + 角色筛选（当前页客户端过滤）
const searchText = ref('')
const roleFilter = ref<string>('')
const filteredUsers = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return users.value.filter((u) => {
    if (roleFilter.value && u.role !== roleFilter.value) return false
    if (!kw) return true
    return u.username.toLowerCase().includes(kw)
  })
})

async function fetch() {
  await usersQuery.refresh()
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  void fetch()
}

function openCreate() {
  form.username = ''
  form.password = ''
  form.role = 'viewer'
  formErrors.username = ''
  formErrors.password = ''
  dialogVisible.value = true
}

function validateForm(): boolean {
  formErrors.username = form.username ? '' : t('user.usernameRequired')
  formErrors.password = form.password ? '' : t('user.passwordRequired')
  return !formErrors.username && !formErrors.password
}

async function handleCreate() {
  if (!validateForm()) return
  submitting.value = true
  try {
    await createUser(form.username, form.password, form.role)
    toast.success(t('common.success'))
    dialogVisible.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function handleEdit(row: User) {
  editingId.value = row.id
  editForm.username = row.username
  editForm.password = ''
  editForm.role = row.role
  formErrors.username = ''
  formErrors.password = ''
  editVisible.value = true
}

async function handleSave() {
  if (!editForm.username) {
    formErrors.username = t('user.usernameRequired')
    return
  }
  submitting.value = true
  try {
    await updateUser(editingId.value, {
      username: editForm.username,
      role: editForm.role,
      ...(editForm.password ? { password_hash: editForm.password } : {}),
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

function confirmDelete(row: User) {
  confirmAction({
    message: t('user.deleteConfirm', { name: row.username }),
    header: t('common.confirmAction'),
    accept: async () => {
      try {
        await deleteUser(row.id)
        toast.success(t('common.success'))
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
}
</script>

<style scoped>
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
</style>
