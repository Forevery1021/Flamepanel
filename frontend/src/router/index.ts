import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useSetupStore } from '@/stores/setup'
import { preloadSavedLocale } from '@/locales'

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
  /** 命令面板搜索权重（数值越大越靠前） */
  weight?: number
  /** 命令面板搜索关键词（可跨语言，如 ['dashboard','监控','ダッシュボード']） */
  keywords?: string[]
}

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    icon?: string
    group?: MenuGroup
    keepAlive?: boolean
    activeMenu?: string
    weight?: number
    keywords?: string[]
  }
}

export const menuRoutes = [
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/DashboardView.vue'),
    meta: { title: 'nav.dashboard', icon: 'oi-gauge', group: 'main' as MenuGroup, keepAlive: true, weight: 100, keywords: ['dashboard', '监控', 'overview', 'home']},
  },
  {
    path: '/users',
    name: 'Users',
    component: () => import('@/views/UsersView.vue'),
    meta: { title: 'nav.users', icon: 'oi-users', group: 'system' as MenuGroup, keepAlive: true, weight: 10, keywords: ['users', '用户', '账号']},
  },
  {
    path: '/nodes',
    name: 'Nodes',
    component: () => import('@/views/NodesView.vue'),
    meta: { title: 'nav.nodes', icon: 'oi-sitemap', group: 'ops' as MenuGroup, keepAlive: true, weight: 50, keywords: ['nodes', '节点', '服务器', 'server']},
  },
  {
    path: '/files',
    name: 'Files',
    component: () => import('@/views/FilesView.vue'),
    meta: { title: 'nav.files', icon: 'oi-folder', group: 'storage' as MenuGroup, keepAlive: true, weight: 60, keywords: ['files', '文件', 'file']},
  },
  {
    path: '/memos',
    name: 'Memos',
    component: () => import('@/views/MemosView.vue'),
    meta: { title: 'nav.memos', icon: 'oi-book', group: 'ops' as MenuGroup, keepAlive: true, weight: 30, keywords: ['memos', '备忘录', '待办', 'todo']},
  },
  {
    path: '/databases',
    name: 'Databases',
    component: () => import('@/views/DatabasesView.vue'),
    meta: { title: 'nav.databases', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true, weight: 40, keywords: ['databases', '数据库', 'db']},
  },
  {
    path: '/websites',
    name: 'Websites',
    component: () => import('@/views/WebsitesView.vue'),
    meta: { title: 'nav.websites', icon: 'oi-globe', group: 'web' as MenuGroup, keepAlive: true, weight: 40, keywords: ['websites', '网站', '站点']},
  },
  {
    path: '/docker',
    name: 'Docker',
    component: () => import('@/views/DockerView.vue'),
    meta: { title: 'nav.docker', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true, weight: 70, keywords: ['docker', '容器', 'container', '镜像', 'image']},
  },
  {
    path: '/plugins',
    name: 'Plugins',
    component: () => import('@/views/PluginsView.vue'),
    meta: { title: 'nav.plugins', icon: 'oi-layers', group: 'apps' as MenuGroup, keepAlive: true, weight: 20, keywords: ['plugins', '插件', '扩展']},
  },
  {
    path: '/app-store',
    name: 'AppStore',
    component: () => import('@/views/AppStoreView.vue'),
    meta: { title: 'nav.appStore', icon: 'oi-box', group: 'apps' as MenuGroup, keepAlive: true, weight: 50, keywords: ['app-store', '应用商店', '安装', '应用', 'apps']},
  },
  {
    path: '/operation-logs',
    name: 'OperationLogs',
    component: () => import('@/views/OperationLogsView.vue'),
    meta: { title: 'nav.operationLogs', icon: 'oi-list-check', group: 'system' as MenuGroup, keepAlive: true, weight: 40, keywords: ['operation-logs', '操作日志', '审计', 'audit']},
  },
  {
    path: '/system-logs',
    name: 'SystemLogs',
    component: () => import('@/views/SystemLogsView.vue'),
    meta: { title: 'nav.systemLogs', icon: 'oi-list', group: 'system' as MenuGroup, keepAlive: true, weight: 35, keywords: ['system-logs', '系统日志', '日志', 'log']},
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: { title: 'nav.settings', icon: 'oi-cog', group: 'system' as MenuGroup, keepAlive: true, weight: 10, keywords: ['settings', '设置', '配置']},
  },
  {
    path: '/firewall',
    name: 'Firewall',
    component: () => import('@/views/FirewallView.vue'),
    meta: { title: 'nav.firewall', icon: 'oi-shield', group: 'ops' as MenuGroup, keepAlive: true, weight: 40, keywords: ['firewall', '防火墙', '安全', 'security']},
  },
  {
    path: '/web-servers',
    name: 'WebServers',
    component: () => import('@/views/WebServersView.vue'),
    meta: { title: 'nav.webServers', icon: 'oi-server', group: 'web' as MenuGroup, keepAlive: true, weight: 25, keywords: ['web-servers', 'Web服务器', 'nginx', '服务器']},
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: () => import('@/views/TerminalView.vue'),
    meta: { title: 'nav.terminal', icon: 'oi-terminal', group: 'ops' as MenuGroup, weight: 55, keywords: ['terminal', '终端', '命令行', 'ssh']},
  },
  {
    path: '/health',
    name: 'Health',
    component: () => import('@/views/HealthView.vue'),
    meta: { title: 'nav.health', icon: 'oi-wave-pulse', group: 'ops' as MenuGroup, keepAlive: true, weight: 45, keywords: ['health', '健康', '状态', 'status']},
  },
  {
    path: '/backups',
    name: 'Backups',
    component: () => import('@/views/BackupView.vue'),
    meta: { title: 'nav.backups', icon: 'oi-save', group: 'storage' as MenuGroup, keepAlive: true, weight: 35, keywords: ['backups', '备份', '恢复', 'backup']},
  },
  {
    path: '/scheduled-tasks',
    name: 'ScheduledTasks',
    component: () => import('@/views/ScheduledTasksView.vue'),
    meta: { title: 'nav.scheduledTasks', icon: 'oi-clock', group: 'ops' as MenuGroup, keepAlive: true, weight: 35, keywords: ['scheduled-tasks', '定时任务', '计划任务', 'cron']},
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/TasksView.vue'),
    meta: { title: 'nav.tasks', icon: 'oi-task', group: 'ops' as MenuGroup, keepAlive: true, weight: 32, keywords: ['tasks', '任务', '进度', 'task', '进度']},
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
      path: '/setup',
      name: 'Setup',
      component: () => import('@/views/setup/SetupView.vue'),
    },
    {
      path: '/',
      component: () => import('@/components/Layout.vue'),
      redirect: '/dashboard',
      children: menuRoutes,
    },
    // F4.2 组件预览页（开发者工具，不进侧边栏）：/dev/ui（仅开发环境注册）
    ...(import.meta.env.DEV
      ? [{
          path: '/dev/ui',
          name: 'DevUi',
          component: () => import('@/views/DevUiView.vue'),
          meta: { title: 'Dev UI', weight: 0 },
        }]
      : []),
  ],
})

router.beforeEach(async (to) => {
  const auth = useAuthStore()
  const setup = useSetupStore()

  // Setup 向导：始终可访问（初始化完成前后端无可认证用户）
  if (to.name === 'Setup') return true

  // B6：初始化状态探测与语言包预载并行（不互相阻塞，避免首屏白屏翻倍）
  const [s] = await Promise.all([setup.ensureStatus(), preloadSavedLocale()])

  if (to.name === 'Login') {
    // 已登录且已初始化：避免登录页闪回
    if (auth.isLoggedIn) return '/'
    // 未初始化：登录页也导向安装向导
    return s?.status === 'in_progress' ? '/setup' : true
  }

  if (s?.status === 'in_progress') {
    // 未初始化：无论目标页面一律导向安装向导
    return '/setup'
  }
  // unattended（无人值守）/ completed：走正常登录流程
  if (!auth.isLoggedIn) {
    return '/login'
  }
})

export default router
