<template>
  <div>
    <div style="display:flex;justify-content:space-between;align-items:center">
      <h2>Users</h2>
      <el-button type="primary" @click="showCreate = true">Create User</el-button>
    </div>
    <el-table :data="users" border stripe v-loading="loading" style="margin-top:16px">
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="username" label="Username" />
      <el-table-column prop="role" label="Role" width="100">
        <template #default="{ row }">
          <el-tag :type="row.role === 'admin' ? 'danger' : row.role === 'operator' ? 'warning' : 'info'" size="small">
            {{ row.role }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="Created" width="180" />
    </el-table>

    <el-dialog v-model="showCreate" title="Create User" width="420px" destroy-on-close>
      <el-form :model="form" label-width="100px" :rules="rules" ref="formRef">
        <el-form-item label="Username" prop="username">
          <el-input v-model="form.username" />
        </el-form-item>
        <el-form-item label="Password" prop="password">
          <el-input v-model="form.password" type="password" show-password />
        </el-form-item>
        <el-form-item label="Role" prop="role">
          <el-select v-model="form.role" style="width:100%">
            <el-option label="admin" value="admin" />
            <el-option label="operator" value="operator" />
            <el-option label="viewer" value="viewer" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">Cancel</el-button>
        <el-button type="primary" @click="handleCreate" :loading="submitting">Create</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { listUsers, createUser } from '@/api/users'
import { ElMessage } from 'element-plus'
import type { User } from '@/types'
import type { FormInstance, FormRules } from 'element-plus'

const users = ref<User[]>([])
const loading = ref(false)
const showCreate = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const form = ref({ username: '', password: '', role: 'viewer' })
const rules: FormRules = {
  username: [{ required: true, message: 'Required', trigger: 'blur' }],
  password: [{ required: true, message: 'Required', trigger: 'blur' }, { min: 4, message: 'Min 4 chars', trigger: 'blur' }],
  role: [{ required: true, message: 'Required', trigger: 'change' }],
}

async function fetch() {
  loading.value = true
  try { users.value = (await listUsers()).data } catch { ElMessage.error('Failed to fetch users') }
  finally { loading.value = false }
}

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    await createUser(form.value.username, form.value.password, form.value.role)
    ElMessage.success('User created')
    showCreate.value = false
    formRef.value?.resetFields()
    fetch()
  } catch (e: any) { ElMessage.error(e.response?.data?.message || 'Create failed') }
  finally { submitting.value = false }
}

onMounted(fetch)
</script>
