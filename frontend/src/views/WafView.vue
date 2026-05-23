<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import {
  ElMessage,
  ElMessageBox,
  ElTable,
  ElTableColumn,
  ElButton,
  ElInput,
  ElDialog,
  ElCard,
  ElTag,
  ElSwitch,
  ElSelect,
  ElOption,
} from 'element-plus'
import type { WafRule } from '@/types'

const rules = ref<WafRule[]>([])
const dialogVisible = ref(false)
const editing = ref(false)
const currentId = ref<number | null>(null)
const form = ref({
  name: '',
  pattern: '',
  target: 'url',
  action: 'block',
  description: '',
})
const loading = ref(false)

const targetOptions = [
  { label: 'URL 路径', value: 'url' },
  { label: '请求头', value: 'header' },
  { label: '请求体', value: 'body' },
  { label: 'Cookie', value: 'cookie' },
]

const actionOptions = [
  { label: '拦截 (Block)', value: 'block' },
  { label: '放行 (Allow)', value: 'allow' },
  { label: '记录 (Log)', value: 'log' },
]

const loadRules = async () => {
  loading.value = true
  try {
    const res = await api.get<WafRule[]>('/waf/rules')
    rules.value = res.data
  } catch (e: any) {
    ElMessage.error('加载 WAF 规则失败')
  } finally {
    loading.value = false
  }
}

const openCreate = () => {
  editing.value = false
  currentId.value = null
  form.value = { name: '', pattern: '', target: 'url', action: 'block', description: '' }
  dialogVisible.value = true
}

const openEdit = (rule: WafRule) => {
  editing.value = true
  currentId.value = rule.id
  form.value = {
    name: rule.name,
    pattern: rule.pattern,
    target: rule.target,
    action: rule.action,
    description: rule.description || '',
  }
  dialogVisible.value = true
}

const saveRule = async () => {
  if (!form.value.name || !form.value.pattern) {
    ElMessage.warning('名称和匹配模式不能为空')
    return
  }
  try {
    if (editing.value && currentId.value) {
      await api.put('/waf/rules/update', { id: currentId.value, ...form.value })
      ElMessage.success('规则更新成功')
    } else {
      await api.post('/waf/rules/create', form.value)
      ElMessage.success('规则创建成功')
    }
    dialogVisible.value = false
    loadRules()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '保存失败')
  }
}

const deleteRule = async (rule: WafRule) => {
  try {
    await ElMessageBox.confirm(`确定要删除规则 "${rule.name}" 吗？`, '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await api.delete('/waf/rules/delete', { params: { id: rule.id } })
    ElMessage.success('规则已删除')
    loadRules()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.response?.data?.message || '删除失败')
    }
  }
}

const toggleRule = async (rule: WafRule) => {
  try {
    await api.post('/waf/rules/toggle', { id: rule.id, enabled: !rule.enabled })
    ElMessage.success(rule.enabled ? '规则已禁用' : '规则已启用')
    loadRules()
  } catch (e: any) {
    ElMessage.error('操作失败')
  }
}

onMounted(loadRules)
</script>

<template>
  <div class="waf-page">
    <ElCard>
      <template #header>
        <div style="display: flex; justify-content: space-between; align-items: center">
          <span>WAF 防火墙规则</span>
          <ElButton type="primary" @click="openCreate">新建规则</ElButton>
        </div>
      </template>

      <ElTable :data="rules" v-loading="loading" stripe>
        <ElTableColumn prop="name" label="规则名称" min-width="160" />
        <ElTableColumn prop="pattern" label="匹配模式" min-width="200">
          <template #default="{ row }">
            <code style="background: #f5f7fa; padding: 2px 6px; border-radius: 3px; font-size: 13px">
              {{ row.pattern }}
            </code>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="target" label="匹配目标" width="100">
          <template #default="{ row }">
            <ElTag size="small">{{ row.target }}</ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="action" label="动作" width="100">
          <template #default="{ row }">
            <ElTag
              size="small"
              :type="row.action === 'block' ? 'danger' : row.action === 'allow' ? 'success' : 'info'"
            >
              {{ row.action }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="description" label="描述" min-width="160">
          <template #default="{ row }">
            {{ row.description || '-' }}
          </template>
        </ElTableColumn>
        <ElTableColumn label="启用" width="80">
          <template #default="{ row }">
            <ElSwitch
              :model-value="row.enabled"
              @change="toggleRule(row)"
              size="small"
            />
          </template>
        </ElTableColumn>
        <ElTableColumn label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" @click="openEdit(row)">编辑</ElButton>
            <ElButton size="small" type="danger" @click="deleteRule(row)">删除</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 新建/编辑规则弹窗 -->
    <ElDialog
      v-model="dialogVisible"
      :title="editing ? '编辑规则' : '新建规则'"
      width="560px"
    >
      <el-form :model="form" label-width="80px">
        <el-form-item label="名称" required>
          <ElInput v-model="form.name" placeholder="例如: Block SQL Injection" />
        </el-form-item>
        <el-form-item label="模式" required>
          <ElInput
            v-model="form.pattern"
            placeholder="正则表达式，例如: (?i)(union\s+select)"
          />
        </el-form-item>
        <el-form-item label="目标">
          <ElSelect v-model="form.target">
            <ElOption
              v-for="opt in targetOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </ElSelect>
        </el-form-item>
        <el-form-item label="动作">
          <ElSelect v-model="form.action">
            <ElOption
              v-for="opt in actionOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </ElSelect>
        </el-form-item>
        <el-form-item label="描述">
          <ElInput v-model="form.description" placeholder="规则说明" />
        </el-form-item>
      </el-form>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveRule">保存</ElButton>
      </template>
    </ElDialog>
  </div>
</template>
