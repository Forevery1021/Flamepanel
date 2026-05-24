<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
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
  Setting,
  Timer,
  Coin,
  ShoppingCart,
  Promotion,
  Clock,
  Bell,
  Box,
  Avatar,
} from '@element-plus/icons-vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const auth = useAuthStore()

const permMap: Record<string, string> = {
  '/dashboard': 'dashboard:view',
  '/file': 'file:manage',
  '/docker': 'docker:manage',
  '/databases': 'database:manage',
  '/website': 'website:manage',
  '/waf': 'waf:manage',
  '/terminal': 'terminal:access',
  '/users': 'users:manage',
  '/roles': 'users:manage',
  '/logs': 'logs:view',
  '/processes': 'process:view',
  '/cleanup': 'system:cleanup',
  '/settings': 'settings:manage',
  '/cron': 'cron:manage',
  '/appstore': 'appstore:manage',
  '/ai': 'ai:access',
  '/nodes': 'nodes:manage',
  '/backup': 'backup:manage',
  '/alerts': 'alerts:manage',
  '/plugins': 'plugins:manage',
}

type MenuItem = { path: string; i18nKey: string; icon: any }

const allMenuItems: MenuItem[] = [
  { path: '/dashboard', i18nKey: 'menu.dashboard', icon: House },
  { path: '/file', i18nKey: 'menu.fileManager', icon: Folder },
  { path: '/docker', i18nKey: 'menu.docker', icon: Grid },
  { path: '/databases', i18nKey: 'menu.databases', icon: Coin },
  { path: '/website', i18nKey: 'menu.websites', icon: Link },
  { path: '/waf', i18nKey: 'menu.waf', icon: Lock },
  { path: '/terminal', i18nKey: 'menu.terminal', icon: Monitor },
  { path: '/users', i18nKey: 'menu.users', icon: User },
  { path: '/roles', i18nKey: 'menu.roles', icon: Avatar },
  { path: '/logs', i18nKey: 'menu.logs', icon: Document },
  { path: '/processes', i18nKey: 'menu.system', icon: Cpu },
  { path: '/cleanup', i18nKey: 'menu.cleanup', icon: Delete },
  { path: '/settings', i18nKey: 'menu.settings', icon: Setting },
  { path: '/cron', i18nKey: 'menu.cronJobs', icon: Timer },
  { path: '/appstore', i18nKey: 'menu.appStore', icon: ShoppingCart },
  { path: '/ai', i18nKey: 'menu.aiAssistant', icon: Promotion },
  { path: '/nodes', i18nKey: 'menu.nodes', icon: Monitor },
  { path: '/backup', i18nKey: 'menu.backup', icon: Clock },
  { path: '/alerts', i18nKey: 'menu.alerts', icon: Bell },
  { path: '/plugins', i18nKey: 'menu.plugins', icon: Box },
]

const menuItems = ref(allMenuItems)

onMounted(async () => {
  if (auth.token && auth.role !== 'admin') {
    try {
      const resp = await fetch('/api/rbac/my-permissions', {
        headers: { Authorization: `Bearer ${auth.token}` },
      })
      if (resp.ok) {
        const data = await resp.json()
        const perms = data.permissions as string[]
        menuItems.value = allMenuItems.filter((item) => {
          const required = permMap[item.path]
          return !required || perms.includes(required)
        })
      }
    } catch {
      // On error, show all items
    }
  }
})

const handleLogout = () => {
  auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="sidebar">
    <div class="logo">
      <h2>Ops Panel</h2>
      <p class="subtitle">Rust O&M Panel</p>
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
        <span>{{ t(item.i18nKey) }}</span>
      </div>
    </div>

    <div class="footer">
      <div class="user">
        <el-icon><User /></el-icon>
        <span>{{ auth.username || 'admin' }}</span>
      </div>
      <el-button type="danger" size="small" @click="handleLogout">
        {{ t('common.logout') }}
      </el-button>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 240px;
  min-width: 240px;
  background: var(--bg-sidebar);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border-right: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  height: 100vh;
  transition: background 0.4s ease, border-color 0.4s ease;
}

.logo {
  padding: 24px;
  border-bottom: 1px solid var(--border-light);
}
.logo h2 {
  margin: 0;
  font-size: 24px;
  font-weight: 800;
  background: linear-gradient(135deg, var(--el-color-primary), var(--el-color-primary-light-3));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.subtitle {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
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
  color: var(--text-regular);
}
.menu-item:hover {
  background: var(--bg-hover);
  color: var(--el-color-primary);
}
.menu-item.active {
  background: var(--el-color-primary);
  color: white;
  box-shadow: 0 4px 12px hsla(var(--hue), var(--sat), 55%, 0.4);
}

.footer {
  padding: 20px;
  border-top: 1px solid var(--border-light);
}

.user {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  color: var(--text-regular);
}
</style>
