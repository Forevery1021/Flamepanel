<template>
  <div>
    <div style="display:flex;justify-content:space-between;align-items:center">
      <h2>Websites</h2>
      <el-button type="primary" @click="showCreate = true">Create Website</el-button>
    </div>
    <el-table :data="websites" border stripe v-loading="loading" style="margin-top:16px">
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="name" label="Name" />
      <el-table-column prop="domain" label="Domain" />
      <el-table-column prop="root_path" label="Root Path" />
      <el-table-column prop="node_id" label="Node" width="60" align="center" />
      <el-table-column prop="status" label="Status" width="80">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'warning'" effect="plain" size="small">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="Created" width="180" />
    </el-table>

    <el-dialog v-model="showCreate" title="Create Website" width="480px" destroy-on-close>
      <el-form :model="form" label-width="110px" :rules="rules" ref="formRef">
        <el-form-item label="Name" prop="name">
          <el-input v-model="form.name" placeholder="e.g. blog" />
        </el-form-item>
        <el-form-item label="Domain" prop="domain">
          <el-input v-model="form.domain" placeholder="e.g. blog.example.com" />
        </el-form-item>
        <el-form-item label="Root Path" prop="root_path">
          <el-input v-model="form.root_path" placeholder="/var/www/blog" />
        </el-form-item>
        <el-form-item label="Node ID" prop="node_id">
          <el-input-number v-model="form.node_id" :min="1" style="width:100%" />
        </el-form-item>
        <el-form-item label="Status" prop="status">
          <el-select v-model="form.status" style="width:100%">
            <el-option label="active" value="active" />
            <el-option label="inactive" value="inactive" />
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
import { listWebsites, createWebsite } from '@/api/websites'
import { ElMessage } from 'element-plus'
import type { Website } from '@/types'
import type { FormInstance, FormRules } from 'element-plus'

const websites = ref<Website[]>([])
const loading = ref(false)
const showCreate = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const form = ref({ name: '', domain: '', root_path: '/var/www', node_id: 1, status: 'active' })
const rules: FormRules = {
  name: [{ required: true, message: 'Required', trigger: 'blur' }],
  domain: [{ required: true, message: 'Required', trigger: 'blur' }],
  root_path: [{ required: true, message: 'Required', trigger: 'blur' }],
}

async function fetch() {
  loading.value = true
  try { websites.value = (await listWebsites()).data } catch { ElMessage.error('Failed to fetch websites') }
  finally { loading.value = false }
}

async function handleCreate() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitting.value = true
  try {
    await createWebsite({ id: 0, ...form.value, created_at: new Date().toISOString() } as Website)
    ElMessage.success('Website created')
    showCreate.value = false
    formRef.value?.resetFields()
    fetch()
  } catch (e: any) { ElMessage.error(e.response?.data?.message || 'Create failed') }
  finally { submitting.value = false }
}

onMounted(fetch)
</script>
