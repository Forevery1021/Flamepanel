<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import type { RoleWithPermissions, Permission, CreateRoleRequest, UpdateRoleRequest } from '@/types'

const roles = ref<RoleWithPermissions[]>([])
const permissions = ref<Permission[]>([])
const loading = ref(false)

const dialogVisible = ref(false)
const dialogTitle = ref('')
const editingRole = ref<RoleWithPermissions | null>(null)
const form = ref({ name: '', description: '', permission_ids: [] as number[] })

async function loadData() {
  loading.value = true
  try {
    const [rolesResp, permsResp] = await Promise.all([
      fetch('/api/rbac/roles'),
      fetch('/api/rbac/permissions'),
    ])
    if (rolesResp.ok) roles.value = await rolesResp.json()
    if (permsResp.ok) permissions.value = await permsResp.json()
  } catch (e: any) {
    ElMessage.error('加载数据失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

function openCreate() {
  dialogTitle.value = '创建角色'
  editingRole.value = null
  form.value = { name: '', description: '', permission_ids: [] }
  dialogVisible.value = true
}

function openEdit(role: RoleWithPermissions) {
  dialogTitle.value = '编辑角色'
  editingRole.value = role
  form.value = {
    name: role.name,
    description: role.description,
    permission_ids: role.permissions.map((p) => p.id),
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!form.value.name) {
    ElMessage.warning('请输入角色名称')
    return
  }
  try {
    if (editingRole.value) {
      const body: UpdateRoleRequest = {
        name: form.value.name,
        description: form.value.description,
        permission_ids: form.value.permission_ids,
      }
      const resp = await fetch(`/api/rbac/roles/${editingRole.value.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
      ElMessage.success('角色已更新')
    } else {
      const body: CreateRoleRequest = {
        name: form.value.name,
        description: form.value.description || undefined,
        permission_ids: form.value.permission_ids,
      }
      const resp = await fetch('/api/rbac/roles', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
      ElMessage.success('角色已创建')
    }
    dialogVisible.value = false
    loadData()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + e.message)
  }
}

async function handleDelete(role: RoleWithPermissions) {
  if (role.is_system) {
    ElMessage.warning('系统内置角色不可删除')
    return
  }
  try {
    await ElMessageBox.confirm(`确定删除角色 "${role.name}"？`, '确认删除', { type: 'warning' })
    const resp = await fetch(`/api/rbac/roles/${role.id}`, { method: 'DELETE' })
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success('角色已删除')
    loadData()
  } catch { /* canceled */ }
}

// Group permissions by resource
const permissionGroups = computed(() => {
  const groups: Record<string, Permission[]> = {}
  for (const p of permissions.value) {
    if (!groups[p.resource]) groups[p.resource] = []
    groups[p.resource].push(p)
  }
  return groups
})

import { computed } from 'vue'

onMounted(loadData)
</script>

<template>
  <div class="roles-view">
    <div class="page-header">
      <h2>角色与权限管理</h2>
      <el-button type="primary" :icon="Plus" @click="openCreate">创建角色</el-button>
    </div>

    <div v-loading="loading">
      <el-card v-for="role in roles" :key="role.id" class="role-card">
        <template #header>
          <div class="role-card-header">
            <div>
              <span class="role-name">{{ role.name }}</span>
              <el-tag v-if="role.is_system" size="small" type="info" style="margin-left: 8px">系统</el-tag>
              <span class="role-desc">{{ role.description }}</span>
            </div>
            <div class="role-actions">
              <el-button :icon="Edit" size="small" @click="openEdit(role)">编辑</el-button>
              <el-button
                :icon="Delete"
                size="small"
                type="danger"
                @click="handleDelete(role)"
                :disabled="role.is_system"
              >
                删除
              </el-button>
            </div>
          </div>
        </template>
        <div class="perm-tags">
          <el-tag
            v-for="perm in role.permissions"
            :key="perm.id"
            size="small"
            type="success"
            effect="plain"
            style="margin: 2px"
          >
            {{ perm.name }}
          </el-tag>
          <span v-if="role.permissions.length === 0" class="no-perms">暂无权限</span>
        </div>
      </el-card>
    </div>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="640px">
      <el-form label-width="80px">
        <el-form-item label="角色名称">
          <el-input v-model="form.name" placeholder="如：operator" />
        </el-form-item>
        <el-form-item label="角色描述">
          <el-input v-model="form.description" placeholder="描述该角色的职责" />
        </el-form-item>
        <el-form-item label="权限分配">
          <div v-for="(perms, resource) in permissionGroups" :key="resource" class="perm-group">
            <div class="perm-resource">{{ resource }}</div>
            <el-checkbox-group v-model="form.permission_ids" class="perm-checkboxes">
              <el-checkbox
                v-for="perm in perms"
                :key="perm.id"
                :label="perm.id"
                :value="perm.id"
              >
                {{ perm.description }}
              </el-checkbox>
            </el-checkbox-group>
          </div>
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
.roles-view {
  padding: 24px;
  height: 100%;
  overflow-y: auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  color: var(--text-primary);
}

.role-card {
  margin-bottom: 12px;
}

.role-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.role-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
}

.role-desc {
  margin-left: 12px;
  font-size: 13px;
  color: var(--text-secondary);
}

.role-actions {
  display: flex;
  gap: 6px;
}

.perm-tags {
  min-height: 24px;
}

.no-perms {
  font-size: 13px;
  color: var(--text-secondary);
  font-style: italic;
}

.perm-group {
  margin-bottom: 12px;
}

.perm-resource {
  font-weight: 600;
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 4px;
  text-transform: uppercase;
}

.perm-checkboxes {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 16px;
}
</style>
