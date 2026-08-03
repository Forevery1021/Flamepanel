<template>
  <div class="view-container">
    <div class="card-header-title">
      <el-button type="primary" @click="dialogVisible = true">{{ t('user.createUser') }}</el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :empty-text="t('common.noData')" :data="users" border stripe>
        <el-table-column prop="id" :label="t('user.id')" width="80" />
        <el-table-column prop="username" :label="t('user.username')" />
        <el-table-column :label="t('user.role')" width="120">
          <template #default="{ row }">
            <el-tag
              :type="row.role === 'admin' ? 'danger' : row.role === 'operator' ? 'warning' : 'info'"
              size="small"
            >
              {{ roleLabel(row.role) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('user.createdAt')" width="180" />
        <el-table-column :label="t('common.operation')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" size="small" text @click="handleEdit(row)">{{
              t('common.edit')
            }}</el-button>
            <el-popconfirm
              :title="t('user.deleteConfirm', { name: row.username })"
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

    <el-dialog v-model="dialogVisible" :title="t('user.createUser')" width="400px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item :label="t('user.username')" prop="username">
          <el-input v-model="form.username" />
        </el-form-item>
        <el-form-item :label="t('user.password')" prop="password">
          <el-input v-model="form.password" type="password" show-password />
        </el-form-item>
        <el-form-item :label="t('user.role')" prop="role">
          <el-select v-model="form.role" class="full-width">
            <el-option :label="t('user.admin')" value="admin" />
            <el-option :label="t('user.operator')" value="operator" />
            <el-option :label="t('user.viewer')" value="viewer" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreate">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="editVisible" :title="t('user.editUser')" width="400px">
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-width="100px">
        <el-form-item :label="t('user.username')" prop="username">
          <el-input v-model="editForm.username" />
        </el-form-item>
        <el-form-item :label="t('user.password')" prop="password">
          <el-input
            v-model="editForm.password"
            type="password"
            show-password
            :placeholder="t('user.passwordOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('user.role')" prop="role">
          <el-select v-model="editForm.role" class="full-width">
            <el-option :label="t('user.admin')" value="admin" />
            <el-option :label="t('user.operator')" value="operator" />
            <el-option :label="t('user.viewer')" value="viewer" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSave">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listUsers, createUser, updateUser, deleteUser } from '@/api/users'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { User } from '@/types'

const { t } = useI18n()
const users = ref<User[]>([])
const loading = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)
const dialogVisible = ref(false)
const editVisible = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editFormRef = ref<FormInstance>()
const editingId = ref(0)

const form = reactive({ username: '', password: '', role: 'viewer' })
const editForm = reactive({ username: '', password: '', role: 'viewer' })
const rules: FormRules = {
  username: [{ required: true, message: t('user.usernameRequired'), trigger: 'blur' }],
  password: [{ required: true, message: t('user.passwordRequired'), trigger: 'blur' }],
  role: [{ required: true, message: t('user.roleRequired'), trigger: 'change' }],
}
const editRules: FormRules = {
  username: [{ required: true, message: t('user.usernameRequired'), trigger: 'blur' }],
  role: [{ required: true, message: t('user.roleRequired'), trigger: 'change' }],
}

function roleLabel(role: string) {
  const map: Record<string, string> = {
    admin: t('user.admin'),
    operator: t('user.operator'),
    viewer: t('user.viewer'),
  }
  return map[role] || role
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

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    await createUser(form.username, form.password, form.role)
    ElMessage.success(t('common.success'))
    dialogVisible.value = false
    form.username = ''
    form.password = ''
    form.role = 'viewer'
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    submitting.value = false
  }
}

function handleEdit(row: User) {
  editingId.value = row.id
  editForm.username = row.username
  editForm.password = ''
  editForm.role = row.role
  editVisible.value = true
}

async function handleSave() {
  const valid = await editFormRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    await updateUser(editingId.value, {
      username: editForm.username,
      role: editForm.role,
      ...(editForm.password ? { password_hash: editForm.password } : {}),
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
    await deleteUser(id)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

onMounted(fetch)
</script>
