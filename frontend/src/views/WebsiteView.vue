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
} from 'element-plus'
import type { Website } from '@/types'

const websites = ref<Website[]>([])
const dialogVisible = ref(false)
const form = ref({
  domain: '',
  root_path: '/www/',
  proxy_port: null as number | null,
  enable_ssl: false,
})
const loading = ref(false)

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
    form.value = { domain: '', root_path: '/www/', proxy_port: null, enable_ssl: false }
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '创建失败')
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
        <div style="display: flex; justify-content: space-between; align-items: center">
          <span>网站管理</span>
          <ElButton type="primary" @click="dialogVisible = true">新建站点</ElButton>
        </div>
      </template>

      <ElTable :data="websites" v-loading="loading" stripe>
        <ElTableColumn prop="domain" label="域名" min-width="180" />
        <ElTableColumn prop="root_path" label="网站目录" min-width="200" />
        <ElTableColumn label="代理端口" width="100">
          <template #default="{ row }">
            {{ row.proxy_port || '-' }}
          </template>
        </ElTableColumn>
        <ElTableColumn label="SSL" width="80">
          <template #default="{ row }">
            <ElTag :type="row.ssl_enabled ? 'success' : 'info'" size="small">
              {{ row.ssl_enabled ? '已开启' : '未开启' }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="状态" width="100">
          <template #default="{ row }">
            <ElSwitch
              :model-value="row.enabled"
              @change="toggleSite(row)"
              active-text="启用"
              inactive-text="禁用"
            />
          </template>
        </ElTableColumn>
        <ElTableColumn prop="created_at" label="创建时间" width="180" />
        <ElTableColumn label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" type="danger" @click="deleteSite(row)">删除</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 新建站点弹窗 -->
    <ElDialog v-model="dialogVisible" title="新建网站" width="520px">
      <el-form :model="form" label-width="100px">
        <el-form-item label="域名" required>
          <ElInput v-model="form.domain" placeholder="example.com" />
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
  </div>
</template>
