<template>
  <LayoutContent :title="t('user.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton variant="primary" icon="oi oi-plus" @click="openCreate">
        {{ t('user.createUser') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable :rows="users" :loading="loading" :first="(currentPage - 1) * pageSize">
        <Column field="id" :header="t('user.id')" style="width: 80px" />
        <Column field="username" :header="t('user.username')" />
        <Column :header="t('user.role')" style="width: 140px">
          <template #body="{ data }">
            <FpTag :severity="roleSeverity(data.role)" :value="roleLabel(data.role)" />
          </template>
        </Column>
        <Column field="created_at" :header="t('user.createdAt')" style="width: 190px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </Column>
        <Column :header="t('common.operation')" style="width: 160px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="link" @click="handleEdit(data)">{{ t('common.edit') }}</FpButton>
              <FpButton variant="link" @click="confirmDelete(data)">{{ t('common.delete') }}</FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
      <Paginator
        v-if="total > pageSize"
        :first="(currentPage - 1) * pageSize"
        :rows="pageSize"
        :total-records="total"
        :rows-per-page-options="[20, 50, 100]"
        @update:first="(f) => goPage(f)"
      />
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
import { reactive, ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import Paginator from 'openvue/paginator'
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
import type { User } from '@/types'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const users = ref<User[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
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

async function fetch() {
  loading.value = true
  try {
    const res = await listUsers(currentPage.value, pageSize.value)
    users.value = res.data.data
    total.value = res.data.total
  } finally {
    loading.value = false
  }
}

function goPage(first: number) {
  currentPage.value = first / pageSize.value + 1
  fetch()
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

onMounted(fetch)
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
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
