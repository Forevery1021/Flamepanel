<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import {
  ElMessage,
  ElMessageBox,
  ElButton,
  ElInput,
  ElDialog,
  ElCard,
  ElTag,
  ElSwitch,
  ElSelect,
  ElOption,
  ElTabs,
  ElTabPane,
} from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import type { WafRule, WafIpRule } from '@/types'

// ─── Pattern Rules ────────────────────────────────────────────────────────────

const rules = ref<WafRule[]>([])
const dialogVisible = ref(false)
const editing = ref(false)
const currentId = ref<number | null>(null)
const form = ref({ name: '', pattern: '', target: 'url', action: 'block', description: '' })
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
  } catch {
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
  form.value = { name: rule.name, pattern: rule.pattern, target: rule.target, action: rule.action, description: rule.description || '' }
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
    await ElMessageBox.confirm(`确定要删除规则 "${rule.name}" 吗？`, '确认删除', { type: 'warning' })
    await api.delete('/waf/rules/delete', { params: { id: rule.id } })
    ElMessage.success('规则已删除')
    loadRules()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.response?.data?.message || '删除失败')
  }
}

const toggleRule = async (rule: WafRule) => {
  try {
    await api.post('/waf/rules/toggle', { id: rule.id, enabled: !rule.enabled })
    ElMessage.success(rule.enabled ? '规则已禁用' : '规则已启用')
    loadRules()
  } catch { ElMessage.error('操作失败') }
}

// ─── IP Rules ─────────────────────────────────────────────────────────────────

const ipRules = ref<WafIpRule[]>([])
const ipDialogVisible = ref(false)
const ipForm = ref({ ip: '', action: 'block', description: '' })
const ipLoading = ref(false)

const loadIpRules = async () => {
  ipLoading.value = true
  try {
    const res = await api.get<WafIpRule[]>('/waf/ip-rules')
    ipRules.value = res.data
  } catch {
    ElMessage.error('加载 IP 规则失败')
  } finally {
    ipLoading.value = false
  }
}

const openIpCreate = () => {
  ipForm.value = { ip: '', action: 'block', description: '' }
  ipDialogVisible.value = true
}

const saveIpRule = async () => {
  if (!ipForm.value.ip) {
    ElMessage.warning('IP 不能为空')
    return
  }
  try {
    await api.post('/waf/ip-rules/create', ipForm.value)
    ElMessage.success('IP 规则创建成功')
    ipDialogVisible.value = false
    loadIpRules()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '创建失败')
  }
}

const deleteIpRule = async (rule: WafIpRule) => {
  try {
    await ElMessageBox.confirm(`确定要删除 IP 规则 "${rule.ip}" 吗？`, '确认删除', { type: 'warning' })
    await api.delete('/waf/ip-rules/delete', { params: { id: rule.id } })
    ElMessage.success('IP 规则已删除')
    loadIpRules()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.response?.data?.message || '删除失败')
  }
}

const toggleIpRule = async (rule: WafIpRule) => {
  try {
    await api.post('/waf/ip-rules/toggle', { id: rule.id, enabled: !rule.enabled })
    ElMessage.success(rule.enabled ? 'IP 规则已禁用' : 'IP 规则已启用')
    loadIpRules()
  } catch { ElMessage.error('操作失败') }
}

onMounted(() => {
  loadRules()
  loadIpRules()
})
</script>

<template>
  <div class="waf-page">
    <ElTabs type="border-card">
      <!-- Pattern Rules Tab -->
      <ElTabPane label="匹配规则">
        <div class="tab-header">
          <span></span>
          <ElButton type="primary" :icon="Plus" @click="openCreate">新建规则</ElButton>
        </div>

        <el-table :data="rules" v-loading="loading" stripe>
          <el-table-column prop="name" label="规则名称" min-width="160" />
          <el-table-column prop="pattern" label="匹配模式" min-width="200">
            <template #default="{ row }">
              <code class="regex-code">{{ row.pattern }}</code>
            </template>
          </el-table-column>
          <el-table-column prop="target" label="匹配目标" width="100">
            <template #default="{ row }">
              <ElTag size="small">{{ row.target }}</ElTag>
            </template>
          </el-table-column>
          <el-table-column prop="action" label="动作" width="100">
            <template #default="{ row }">
              <ElTag size="small" :type="row.action === 'block' ? 'danger' : row.action === 'allow' ? 'success' : 'info'">
                {{ row.action }}
              </ElTag>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="160">
            <template #default="{ row }">{{ row.description || '-' }}</template>
          </el-table-column>
          <el-table-column label="启用" width="80">
            <template #default="{ row }">
              <ElSwitch :model-value="row.enabled" @change="toggleRule(row)" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="180" fixed="right">
            <template #default="{ row }">
              <ElButton size="small" :icon="Edit" @click="openEdit(row)">编辑</ElButton>
              <ElButton size="small" type="danger" :icon="Delete" @click="deleteRule(row)">删除</ElButton>
            </template>
          </el-table-column>
        </el-table>
      </ElTabPane>

      <!-- IP Rules Tab -->
      <ElTabPane label="IP 黑白名单">
        <div class="tab-header">
          <span></span>
          <ElButton type="primary" :icon="Plus" @click="openIpCreate">添加 IP 规则</ElButton>
        </div>

        <el-table :data="ipRules" v-loading="ipLoading" stripe>
          <el-table-column prop="ip" label="IP / CIDR" min-width="200" />
          <el-table-column prop="action" label="动作" width="100">
            <template #default="{ row }">
              <ElTag size="small" :type="row.action === 'block' ? 'danger' : 'success'">
                {{ row.action === 'block' ? '拦截' : '放行' }}
              </ElTag>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="200">
            <template #default="{ row }">{{ row.description || '-' }}</template>
          </el-table-column>
          <el-table-column label="启用" width="80">
            <template #default="{ row }">
              <ElSwitch :model-value="row.enabled" @change="toggleIpRule(row)" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="created_at" label="创建时间" width="180" />
          <el-table-column label="操作" width="100" fixed="right">
            <template #default="{ row }">
              <ElButton size="small" type="danger" :icon="Delete" @click="deleteIpRule(row)">删除</ElButton>
            </template>
          </el-table-column>
        </el-table>
      </ElTabPane>
    </ElTabs>

    <!-- Pattern Rule Dialog -->
    <ElDialog v-model="dialogVisible" :title="editing ? '编辑规则' : '新建规则'" width="560px">
      <el-form :model="form" label-width="80px">
        <el-form-item label="名称" required>
          <ElInput v-model="form.name" placeholder="例如: Block SQL Injection" />
        </el-form-item>
        <el-form-item label="模式" required>
          <ElInput v-model="form.pattern" placeholder="正则表达式" />
        </el-form-item>
        <el-form-item label="目标">
          <ElSelect v-model="form.target">
            <ElOption v-for="opt in targetOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
          </ElSelect>
        </el-form-item>
        <el-form-item label="动作">
          <ElSelect v-model="form.action">
            <ElOption v-for="opt in actionOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
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

    <!-- IP Rule Dialog -->
    <ElDialog v-model="ipDialogVisible" title="添加 IP 规则" width="480px">
      <el-form :model="ipForm" label-width="80px">
        <el-form-item label="IP 地址" required>
          <ElInput v-model="ipForm.ip" placeholder="192.168.1.1 或 10.0.0.0/24" />
        </el-form-item>
        <el-form-item label="动作">
          <ElSelect v-model="ipForm.action">
            <ElOption label="拦截 (Block)" value="block" />
            <ElOption label="放行 (Allow)" value="allow" />
          </ElSelect>
        </el-form-item>
        <el-form-item label="描述">
          <ElInput v-model="ipForm.description" placeholder="规则说明" />
        </el-form-item>
      </el-form>
      <template #footer>
        <ElButton @click="ipDialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveIpRule">保存</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.waf-page {
  max-width: 1400px;
}
.tab-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.regex-code {
  background: #f5f7fa;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 13px;
}
</style>
