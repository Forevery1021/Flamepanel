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
} from 'element-plus'
import { Plus, Delete, Refresh, Key } from '@element-plus/icons-vue'
import type { Website } from '@/types'

const engines = [
  { label: 'Nginx', value: 'nginx' },
  { label: 'Apache', value: 'apache' },
  { label: 'Lighttpd', value: 'lighttpd' },
  { label: 'OpenResty', value: 'openresty' },
]

const websites = ref<Website[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const sslVisible = ref(false)
const currentSite = ref<Website | null>(null)
const sslForm = ref({ cert_path: '', key_path: '' })

const form = ref({
  domain: '',
  root_path: '/www/',
  proxy_port: null as number | null,
  enable_ssl: false,
  engine: 'nginx',
})

const loadWebsites = async () => {
  loading.value = true
  try {
    const res = await api.get('/website/list')
    websites.value = res.data.sites
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载网站列表失败')
  } finally {
    loading.value = false
  }
}

const createWebsite = async () => {
  if (!form.value.domain) {
    ElMessage.warning('请输入域名')
    return
  }
  try {
    await api.post('/website/create', form.value)
    ElMessage.success('站点创建成功')
    dialogVisible.value = false
    loadWebsites()
    form.value = { domain: '', root_path: '/www/', proxy_port: null, enable_ssl: false, engine: 'nginx' }
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '创建失败')
  }
}

const openSslDialog = (site: Website) => {
  currentSite.value = site
  sslForm.value = { cert_path: site.ssl_cert_path || '', key_path: site.ssl_key_path || '' }
  sslVisible.value = true
}

const enableSsl = async () => {
  if (!currentSite.value) return
  try {
    await api.post('/website/ssl', { id: currentSite.value.id, ...sslForm.value })
    ElMessage.success('SSL 配置成功')
    sslVisible.value = false
    loadWebsites()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || 'SSL 配置失败')
  }
}

const toggleSite = async (site: Website) => {
  try {
    await api.post('/website/toggle', { id: site.id, enabled: !site.enabled })
    ElMessage.success(site.enabled ? '站点已禁用' : '站点已启用')
    loadWebsites()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '操作失败')
  }
}

const reloadEngine = async (engine: string) => {
  try {
    await api.post('/website/reload', null, { params: { engine } })
    ElMessage.success(`${engine} 已重载`)
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '重载失败')
  }
}

const deleteSite = async (site: Website) => {
  try {
    await ElMessageBox.confirm(`确定要删除站点 ${site.domain} 吗？`, '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await api.delete('/website/delete', { params: { id: site.id } })
    ElMessage.success('站点已删除')
    loadWebsites()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.response?.data?.message || '删除失败')
    }
  }
}

onMounted(loadWebsites)
</script>

<template>
  <div class="website-page">
    <ElCard>
      <template #header>
        <div class="header-row">
          <span>网站管理</span>
          <div class="header-actions">
            <ElButton
              v-for="e in engines"
              :key="e.value"
              :icon="Refresh"
              size="small"
              @click="reloadEngine(e.value)"
            >
              重载 {{ e.label }}
            </ElButton>
            <ElButton type="primary" :icon="Plus" @click="dialogVisible = true">新建站点</ElButton>
          </div>
        </div>
      </template>

      <el-table :data="websites" v-loading="loading" stripe>
        <el-table-column prop="domain" label="域名" min-width="180" />
        <el-table-column prop="root_path" label="网站目录" min-width="200" />
        <el-table-column label="引擎" width="100">
          <template #default="{ row }">
            <ElTag type="primary" size="small">{{ row.engine?.toUpperCase() }}</ElTag>
          </template>
        </el-table-column>
        <el-table-column label="代理端口" width="100">
          <template #default="{ row }">
            {{ row.proxy_port || '-' }}
          </template>
        </el-table-column>
        <el-table-column label="SSL" width="150">
          <template #default="{ row }">
            <div style="display:flex;align-items:center;gap:8px">
              <ElTag :type="row.ssl_enabled ? 'success' : 'info'" size="small">
                {{ row.ssl_enabled ? '已开启' : '未开启' }}
              </ElTag>
              <ElButton :icon="Key" size="small" circle @click="openSslDialog(row)" />
            </div>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <ElSwitch
              :model-value="row.enabled"
              @change="toggleSite(row)"
              active-text="启用"
              inactive-text="禁用"
            />
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180" />
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" type="danger" :icon="Delete" @click="deleteSite(row)">删除</ElButton>
          </template>
        </el-table-column>
      </el-table>
    </ElCard>

    <!-- 新建站点弹窗 -->
    <ElDialog v-model="dialogVisible" title="新建网站" width="520px">
      <el-form :model="form" label-width="100px">
        <el-form-item label="域名" required>
          <ElInput v-model="form.domain" placeholder="example.com" />
        </el-form-item>
        <el-form-item label="引擎">
          <ElSelect v-model="form.engine" style="width:100%">
            <ElOption
              v-for="e in engines"
              :key="e.value"
              :label="e.label"
              :value="e.value"
            />
          </ElSelect>
        </el-form-item>
        <el-form-item label="网站目录">
          <ElInput v-model="form.root_path" placeholder="/www/example" />
        </el-form-item>
        <el-form-item label="代理端口">
          <ElInput v-model.number="form.proxy_port" type="number" placeholder="留空为静态网站" />
        </el-form-item>
        <el-form-item label="启用 SSL">
          <el-switch v-model="form.enable_ssl" />
        </el-form-item>
      </el-form>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="createWebsite">确定创建</ElButton>
      </template>
    </ElDialog>

    <!-- SSL 配置弹窗 -->
    <ElDialog v-model="sslVisible" title="SSL 证书配置" width="520px">
      <el-form :model="sslForm" label-width="120px">
        <el-form-item label="证书路径 (.pem/.crt)">
          <ElInput v-model="sslForm.cert_path" placeholder="/etc/ssl/certs/example.pem" />
        </el-form-item>
        <el-form-item label="密钥路径 (.key)">
          <ElInput v-model="sslForm.key_path" placeholder="/etc/ssl/private/example.key" />
        </el-form-item>
      </el-form>
      <template #footer>
        <ElButton @click="sslVisible = false">取消</ElButton>
        <ElButton type="primary" @click="enableSsl">保存</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
