<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api/client'
import { ElMessage, ElTable, ElTableColumn, ElButton, ElInput, ElDialog } from 'element-plus'

interface Website {
  id: number
  domain: string
  root: string
  port: number
  status: string
}

const websites = ref<Website[]>([])
const dialogVisible = ref(false)
const form = ref({
  domain: '',
  root: '/www/default',
  port: 80,
  ssl: false
})
const loading = ref(false)

const loadWebsites = async () => {
  loading.value = true
  try {
    // 暂时使用模拟数据，后续替换为真实接口
    websites.value = [
      { id: 1, domain: 'example.com', root: '/www/example', port: 80, status: 'running' },
      { id: 2, domain: 'test.com', root: '/www/test', port: 8080, status: 'stopped' },
    ]
  } catch (e) {
    ElMessage.error('加载网站列表失败')
  } finally {
    loading.value = false
  }
}

const createWebsite = async () => {
  try {
    await api.post('/website/create', form.value)
    ElMessage.success('站点创建成功')
    dialogVisible.value = false
    loadWebsites()
    // 清空表单
    form.value = { domain: '', root: '/www/default', port: 80, ssl: false }
  } catch (e) {
    ElMessage.error('创建失败')
  }
}

onMounted(loadWebsites)
</script>

<template>
  <ElCard>
    <template #header>
      <div class="flex justify-between items-center">
        <span>网站管理</span>
        <ElButton type="primary" @click="dialogVisible = true">新建站点</ElButton>
      </div>
    </template>

    <ElTable :data="websites" v-loading="loading" stripe>
      <ElTableColumn prop="domain" label="域名" />
      <ElTableColumn prop="root" label="网站目录" />
      <ElTableColumn prop="port" label="端口" width="100" />
      <ElTableColumn prop="status" label="状态" width="120">
        <template #default="{ row }">
          <el-tag :type="row.status === 'running' ? 'success' : 'danger'">
            {{ row.status === 'running' ? '运行中' : '已停止' }}
          </el-tag>
        </template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="200">
        <template #default>
          <ElButton type="primary" size="small">配置</ElButton>
          <ElButton type="danger" size="small">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>
  </ElCard>

  <!-- 新建站点弹窗 -->
  <ElDialog v-model="dialogVisible" title="新建网站" width="500">
    <el-form :model="form" label-width="80px">
      <el-form-item label="域名">
        <ElInput v-model="form.domain" placeholder="example.com" />
      </el-form-item>
      <el-form-item label="目录">
        <ElInput v-model="form.root" placeholder="/www/example" />
      </el-form-item>
      <el-form-item label="端口">
        <ElInput v-model.number="form.port" type="number" />
      </el-form-item>
    </el-form>
    <template #footer>
      <ElButton @click="dialogVisible = false">取消</ElButton>
      <ElButton type="primary" @click="createWebsite">确定创建</ElButton>
    </template>
  </ElDialog>
</template>