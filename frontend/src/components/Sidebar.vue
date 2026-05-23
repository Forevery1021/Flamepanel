<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import {
  House,
  Folder,
  Grid,
  Monitor,
  Link,
  Lock,
  User,
  Document,
  Cpu,
  Delete,
} from '@element-plus/icons-vue'

const router = useRouter()
const route = useRoute()
const auth = useAuthStore()

const menuItems = [
  { path: '/dashboard', name: '仪表盘', icon: House },
  { path: '/file', name: '文件管理', icon: Folder },
  { path: '/docker', name: 'Docker 管理', icon: Grid },
  { path: '/website', name: '网站管理', icon: Link },
  { path: '/waf', name: 'WAF 防火墙', icon: Lock },
  { path: '/terminal', name: 'Web 终端', icon: Monitor },
  { path: '/users', name: '用户管理', icon: User },
  { path: '/logs', name: '操作日志', icon: Document },
  { path: '/processes', name: '进程管理', icon: Cpu },
  { path: '/cleanup', name: '系统清理', icon: Delete },
]

const handleLogout = () => {
  auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="sidebar">
    <div class="logo">
      <h2>Ops Panel</h2>
      <p class="subtitle">Rust 运维面板</p>
    </div>

    <div class="menu">
      <div
        v-for="item in menuItems"
        :key="item.path"
        class="menu-item"
        :class="{ active: route.path === item.path }"
        @click="router.push(item.path)"
      >
        <el-icon class="icon"><component :is="item.icon" /></el-icon>
        <span>{{ item.name }}</span>
      </div>
    </div>

    <div class="footer">
      <div class="user">
        <el-icon><User /></el-icon>
        <span>{{ auth.username || 'admin' }}</span>
      </div>
      <el-button type="danger" size="small" @click="handleLogout">
        退出登录
      </el-button>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 240px;
  min-width: 240px;
  background: #ffffff;
  border-right: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.logo {
  padding: 24px;
  border-bottom: 1px solid #e4e7ed;
}
.logo h2 {
  margin: 0;
  font-size: 24px;
  color: #409eff;
}
.subtitle {
  margin: 4px 0 0;
  font-size: 12px;
  color: #909399;
}

.menu {
  flex: 1;
  padding: 16px 8px;
  overflow-y: auto;
}
.menu-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  margin: 4px 8px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  color: #606266;
}
.menu-item:hover {
  background: #f0f7ff;
  color: #409eff;
}
.menu-item.active {
  background: #409eff;
  color: white;
}

.footer {
  padding: 20px;
  border-top: 1px solid #e4e7ed;
}

.user {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  color: #606266;
}
</style>
