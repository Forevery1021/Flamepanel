import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

/** 菜单分组 key（AppSidebar 按此聚合） */
export type MenuGroup = 'main' | 'web' | 'apps' | 'storage' | 'ops' | 'system'

export interface RouteMeta {
  /** i18n key，如 nav.dashboard */
  title: string
  /** openicons 图标类，如 oi-gauge */
  icon?: string
  /** 所属菜单分组（空则不进侧边栏） */
  group?: MenuGroup
  /** 是否加入 keep-alive 缓存 */
  keepAlive?: boolean
  /** 详情页归属的菜单高亮路径 */
  activeMenu?: string
}

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    icon?: string
    group?: MenuGroup
    keepAlive?: boolean
    activeMenu?: string
  }
}

export const menuRoutes = [
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/DashboardView.vue'),
    meta: { title: 'nav.dashboard', icon: 'oi-gauge', group: 'main' as MenuGroup, keepAlive: true },
  },
  {
    path: '/users',
    name: 'Users',
    component: () => import('@/views/UsersView.vue'),
    meta: { title: 'nav.users', icon: 'oi-users', group: 'system' as MenuGroup, keepAlive: true },
  },
  {
    path: '/nodes',
    name: 'Nodes',
    component: () => import('@/views/NodesView.vue'),
    meta: { title: 'nav.nodes', icon: 'oi-sitemap', group: 'ops' as MenuGroup, keepAlive: true },
  },
  {
    path: '/files',
    name: 'Files',
    component: () => import('@/views/FilesView.vue'),
    meta: { title: 'nav.files', icon: 'oi-folder', group: 'storage' as MenuGroup, keepAlive: true },
  },
  {
    path: '/memos',
    name: 'Memos',
    component: () => import('@/views/MemosView.vue'),
    meta: { title: 'nav.memos', icon: 'oi-book', group: 'ops' as MenuGroup, keepAlive: true },
  },
  {
    path: '/databases',
    name: 'Databases',
    component: () => import('@/views/DatabasesView.vue'),
    meta: { title: 'nav.databases', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true },
  },
  {
    path: '/websites',
    name: 'Websites',
    component: () => import('@/views/WebsitesView.vue'),
    meta: { title: 'nav.websites', icon: 'oi-globe', group: 'web' as MenuGroup, keepAlive: true },
  },
  {
    path: '/docker',
    name: 'Docker',
    component: () => import('@/views/DockerView.vue'),
    meta: { title: 'nav.docker', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true },
  },
  {
    path: '/plugins',
    name: 'Plugins',
    component: () => import('@/views/PluginsView.vue'),
    meta: { title: 'nav.plugins', icon: 'oi-layers', group: 'apps' as MenuGroup, keepAlive: true },
  },
  {
    path: '/app-store',
    name: 'AppStore',
    component: () => import('@/views/AppStoreView.vue'),
    meta: { title: 'nav.appStore', icon: 'oi-box', group: 'apps' as MenuGroup, keepAlive: true },
  },
  {
    path: '/operation-logs',
    name: 'OperationLogs',
    component: () => import('@/views/OperationLogsView.vue'),
    meta: { title: 'nav.operationLogs', icon: 'oi-list-check', group: 'system' as MenuGroup, keepAlive: true },
  },
  {
    path: '/system-logs',
    name: 'SystemLogs',
    component: () => import('@/views/SystemLogsView.vue'),
    meta: { title: 'nav.systemLogs', icon: 'oi-list', group: 'system' as MenuGroup, keepAlive: true },
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: { title: 'nav.settings', icon: 'oi-cog', group: 'system' as MenuGroup, keepAlive: true },
  },
  {
    path: '/firewall',
    name: 'Firewall',
    component: () => import('@/views/FirewallView.vue'),
    meta: { title: 'nav.firewall', icon: 'oi-shield', group: 'ops' as MenuGroup, keepAlive: true },
  },
  {
    path: '/web-servers',
    name: 'WebServers',
    component: () => import('@/views/WebServersView.vue'),
    meta: { title: 'nav.webServers', icon: 'oi-server', group: 'web' as MenuGroup, keepAlive: true },
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: () => import('@/views/TerminalView.vue'),
    meta: { title: 'nav.terminal', icon: 'oi-terminal', group: 'ops' as MenuGroup },
  },
  {
    path: '/health',
    name: 'Health',
    component: () => import('@/views/HealthView.vue'),
    meta: { title: 'nav.health', icon: 'oi-wave-pulse', group: 'ops' as MenuGroup, keepAlive: true },
  },
  {
    path: '/backups',
    name: 'Backups',
    component: () => import('@/views/BackupView.vue'),
    meta: { title: 'nav.backups', icon: 'oi-save', group: 'storage' as MenuGroup, keepAlive: true },
  },
  {
    path: '/scheduled-tasks',
    name: 'ScheduledTasks',
    component: () => import('@/views/ScheduledTasksView.vue'),
    meta: { title: 'nav.scheduledTasks', icon: 'oi-clock', group: 'ops' as MenuGroup, keepAlive: true },
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/LoginView.vue'),
    },
    {
      path: '/',
      component: () => import('@/components/Layout.vue'),
      redirect: '/dashboard',
      children: menuRoutes,
    },
  ],
})

router.beforeEach((to, _from) => {
  const auth = useAuthStore()
  if (to.name !== 'Login' && !auth.isLoggedIn) {
    return '/login'
  }
})

export default router
