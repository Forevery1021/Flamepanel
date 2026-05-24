<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete, Key } from '@element-plus/icons-vue'
import api from '@/api/client'
import type { User, Role } from '@/types'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const users = ref<User[]>([])
const roles = ref<Role[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const dialogTitle = ref('')
const form = ref({ username: '', password: '', role: 'viewer' })

const fetchUsers = async () => {
  loading.value = true
  try {
    const { data } = await api.get<User[]>('/users/list')
    users.value = data
  } catch {
    ElMessage.error('获取用户列表失败')
  } finally {
    loading.value = false
  }
}

const fetchRoles = async () => {
  try {
    if (auth.role === 'admin') {
      const resp = await fetch('/api/rbac/roles', {
        headers: { Authorization: `Bearer ${auth.token}` },
      })
      if (resp.ok) roles.value = await resp.json()
    }
  } catch { /* ignore */ }
}

const openCreate = () => {
  dialogTitle.value = '创建用户'
  form.value = { username: '', password: '', role: roles.value[0]?.name || 'viewer' }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!form.value.username || !form.value.password) {
    ElMessage.warning('请填写完整信息')
    return
  }
  try {
    await api.post('/auth/register', form.value)
    ElMessage.success('用户创建成功')
    dialogVisible.value = false
    fetchUsers()
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || '创建失败')
  }
}

const handleDelete = async (user: User) => {
  if (user.username === auth.username) {
    ElMessage.warning('不能删除自己')
    return
  }
  try {
    await ElMessageBox.confirm(`确定删除用户 "${user.username}"？`, '确认删除', {
      type: 'warning',
    })
    await api.delete('/users/delete', { params: { id: user.id } })
    ElMessage.success('用户已删除')
    fetchUsers()
  } catch { /* canceled */ }
}

const handleRoleChange = async (user: User, newRole: string) => {
  try {
    await fetch(`/api/rbac/assign-role`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${auth.token}`,
      },
      body: JSON.stringify({ user_id: user.id, role: newRole }),
    })
    ElMessage.success('角色更新成功')
    fetchUsers()
  } catch {
    ElMessage.error('角色更新失败')
  }
}

const handleResetPassword = async (user: User) => {
  try {
    const { value } = await ElMessageBox.prompt('请输入新密码（至少6位）', '重置密码', {
      inputType: 'password',
      inputValidator: (v) => v.length >= 6 ? true : '密码至少6位',
    })
    await api.put('/users/reset-password', null, { params: { id: user.id, password: value } })
    ElMessage.success('密码已重置')
  } catch { /* canceled */ }
}

onMounted(() => { fetchUsers(); fetchRoles() })
</script>

<template>
  <div class="users-page">
    <div class="page-header">
      <h3>用户管理</h3>
      <el-button type="primary" :icon="Plus" @click="openCreate" v-if="auth.role === 'admin'">
        创建用户
      </el-button>
    </div>

    <el-table :data="users" v-loading="loading" stripe>
      <el-table-column prop="id" label="ID" width="80" />
      <el-table-column prop="username" label="用户名" />
      <el-table-column prop="role" label="角色">
        <template #default="{ row }">
          <el-select
            :model-value="row.role"
            @change="(v: string) => handleRoleChange(row, v)"
            size="small"
            style="width: 100px"
            :disabled="auth.role !== 'admin' || row.username === auth.username"
          >
            <el-option
              v-for="r in roles"
              :key="r.name"
              :label="r.description || r.name"
              :value="r.name"
            />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="180" />
      <el-table-column prop="last_login" label="最后登录" width="180">
        <template #default="{ row }">
          {{ row.last_login || '从未登录' }}
        </template>
      </el-table-column>
      <el-table-column label="操作" width="180" v-if="auth.role === 'admin'">
        <template #default="{ row }">
          <el-button :icon="Key" size="small" type="warning" @click="handleResetPassword(row)">
            重置密码
          </el-button>
          <el-button
            :icon="Delete"
            size="small"
            type="danger"
            @click="handleDelete(row)"
            :disabled="row.username === auth.username"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="420px">
      <el-form label-width="80px">
        <el-form-item label="用户名">
          <el-input v-model="form.username" placeholder="至少3位" />
        </el-form-item>
        <el-form-item label="密码">
          <el-input v-model="form.password" type="password" placeholder="至少6位" show-password />
        </el-form-item>
        <el-form-item label="角色">
          <el-select v-model="form.role" style="width: 100%">
            <el-option
              v-for="r in roles"
              :key="r.name"
              :label="r.description || r.name"
              :value="r.name"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确认</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.users-page {
  max-width: 1200px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-header h3 {
  margin: 0;
}
</style>
