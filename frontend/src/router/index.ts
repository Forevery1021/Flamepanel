import {
  createRouter,
  createWebHistory,
  type RouteLocationNormalized,
  type RouteLocationRaw,
} from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useSetupStore } from '@/stores/setup'
import { preloadSavedLocale } from '@/locales'
import { hasRoutePermission } from '@/directives/permission'
import { emitNotice } from '@/utils/notice'
import { recordRecentRoute } from '@/utils/recentRoutes'

/** 菜单分组 key（AppSidebar 按此聚合） */
export type MenuGroup = 'main' | 'web' | 'apps' | 'storage' | 'ops' | 'system'

export interface RouteMeta {
  /** i18n key，如 nav.dashboard */
  title: string
  /** openicons 图标类，如 oi-gauge */
  icon?: string
  /** 所属菜单分组（空则不进侧边栏） */
  group?: MenuGroup
  /** 一级菜单固定位置：`bottom` = 钉在侧栏导航底部（如「面板设置」） */
  pinned?: 'bottom'
  /** 是否加入 keep-alive 缓存 */
  keepAlive?: boolean
  /**
   * 视图 SFC 组件名（keep-alive include 用）。缺省为 `${route.name}View`；
   * 仅当文件名与路由名不一致时显式声明（如 `/runtimes` → `RuntimeView.vue`）。
   */
  viewName?: string
  /** 详情页归属的菜单高亮路径 */
  activeMenu?: string
  /** 命令面板搜索权重（数值越大越靠前） */
  weight?: number
  /** 命令面板搜索关键词（可跨语言，如 ['dashboard','监控','ダッシュボード']） */
  keywords?: string[]
  /**
   * 页面级权限点（resource:action，与后端 `api/permissions.rs` 字面对齐）。
   *
   * 两个消费方共用判据 `directives/permission.ts::hasRoutePermission`（守卫）
   * 与 `isRouteVisibleInMenu`（侧栏 / ⌘K），避免出现「菜单可见但守卫拒绝」的死路：
   * 1. 路由守卫 `checkPagePermission`——直接输入 URL 时按此判定，无权限重定向 `/dashboard`；
   * 2. 侧栏 `AppSidebar` / ⌘K `config/commands.ts`——据此隐藏入口。
   *
   * 缺省（undefined）= 任意已登录角色可进入，仅限以下场景：
   * - 无写能力诉求的落地页（`/dashboard`，同时作为守卫的兜底重定向目标，
   *   若给它加权限会造成「重定向目标本身被拒」的循环）；
   * - 页面能力无法用单一权限点表达（`/settings`：含仅需登录的自助改密 +
   *   备份/DNS/主题等分属不同资源的页签；`/operation-logs`：后端对 viewer/operator
   *   的 `operation_log:read` 实际放行）；
   * - 只读公开资料（`/api-docs`：数据源 `/api/openapi.json` 本身免鉴权，
   *   每个端点的所需权限已在页内以 `x-permission` 标注）。
   */
  perm?: string
}

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    icon?: string
    group?: MenuGroup
    pinned?: 'bottom'
    keepAlive?: boolean
    viewName?: string
    activeMenu?: string
    weight?: number
    keywords?: string[]
    perm?: string
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
    meta: { title: 'nav.users', icon: 'oi-users', group: 'system' as MenuGroup, keepAlive: true, weight: 10, keywords: ['users', '用户', '账号'], perm: 'user:read' },
  },
  {
    path: '/nodes',
    name: 'Nodes',
    component: () => import('@/views/NodesView.vue'),
    meta: { title: 'nav.nodes', icon: 'oi-sitemap', group: 'ops' as MenuGroup, keepAlive: true, weight: 50, keywords: ['nodes', '节点', '服务器', 'server'], perm: 'node:read' },
  },
  {
    path: '/files',
    name: 'Files',
    component: () => import('@/views/FilesView.vue'),
    meta: { title: 'nav.files', icon: 'oi-folder', group: 'storage' as MenuGroup, keepAlive: true, weight: 60, keywords: ['files', '文件', 'file'], perm: 'file:read' },
  },
  {
    path: '/memos',
    name: 'Memos',
    component: () => import('@/views/MemosView.vue'),
    meta: { title: 'nav.memos', icon: 'oi-book', group: 'ops' as MenuGroup, keepAlive: true, weight: 30, keywords: ['memos', '备忘录', '待办', 'todo'], perm: 'memo:read' },
  },
  {
    path: '/databases',
    name: 'Databases',
    component: () => import('@/views/DatabasesView.vue'),
    meta: { title: 'nav.databases', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true, weight: 40, keywords: ['databases', '数据库', 'db'], perm: 'database:read' },
  },
  {
    path: '/websites',
    name: 'Websites',
    component: () => import('@/views/WebsitesView.vue'),
    meta: { title: 'nav.websites', icon: 'oi-globe', group: 'web' as MenuGroup, keepAlive: true, weight: 40, keywords: ['websites', '网站', '站点'], perm: 'website:read' },
  },
  {
    path: '/docker',
    name: 'Docker',
    component: () => import('@/views/DockerView.vue'),
    meta: { title: 'nav.docker', icon: 'oi-database', group: 'storage' as MenuGroup, keepAlive: true, weight: 70, keywords: ['docker', '容器', 'container', '镜像', 'image'], perm: 'docker:read' },
  },
  {
    path: '/plugins',
    name: 'Plugins',
    component: () => import('@/views/PluginsView.vue'),
    meta: { title: 'nav.plugins', icon: 'oi-layers', group: 'apps' as MenuGroup, keepAlive: true, weight: 20, keywords: ['plugins', '插件', '扩展'], perm: 'plugin:read' },
  },
  {
    path: '/app-store',
    name: 'AppStore',
    component: () => import('@/views/AppStoreView.vue'),
    meta: { title: 'nav.appStore', icon: 'oi-box', group: 'apps' as MenuGroup, keepAlive: true, weight: 50, keywords: ['app-store', '应用商店', '安装', '应用', 'apps'], perm: 'app_store:read' },
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
    meta: { title: 'nav.systemLogs', icon: 'oi-list', group: 'system' as MenuGroup, keepAlive: true, weight: 35, keywords: ['system-logs', '系统日志', '日志', 'log'], perm: 'log:read' },
  },
  {
    path: '/ai-center',
    name: 'AiCenter',
    component: () => import('@/views/AiCenterView.vue'),
    // M-AI-1 PR-B：MCP 凭证签发 / 吊销、客户端接入配置、工具调用审计（Doc/34）
    meta: { title: 'nav.aiCenter', icon: 'oi-sparkles', group: 'system' as MenuGroup, keepAlive: true, weight: 42, keywords: ['ai', 'mcp', 'agent', 'claude', 'cursor', 'AI 接入', '智能体', '凭证', 'token'], perm: 'mcp:read' },
  },
  {
    path: '/api-docs',
    name: 'ApiDocs',
    component: () => import('@/views/ApiDocsView.vue'),
    // Doc/32 §6：文档本身免登录可读（/api/openapi.json），不设 perm——任何登录用户都能查阅
    meta: { title: 'nav.apiDocs', icon: 'oi-code', group: 'system' as MenuGroup, keepAlive: true, weight: 45, keywords: ['api', 'api-docs', 'openapi', '接口', '文档', 'fpctl'] },
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    // UX-3：面板设置提升为侧边栏一级菜单（固定在导航底部，不再藏在「系统管理」分组内）
    meta: { title: 'nav.settings', icon: 'oi-cog', group: 'main' as MenuGroup, pinned: 'bottom' as const, keepAlive: true, weight: 90, keywords: ['settings', '设置', '配置', 'preferences']},
  },
  {
    path: '/firewall',
    name: 'Firewall',
    component: () => import('@/views/FirewallView.vue'),
    meta: { title: 'nav.firewall', icon: 'oi-shield', group: 'ops' as MenuGroup, keepAlive: true, weight: 40, keywords: ['firewall', '防火墙', '安全', 'security'], perm: 'firewall:read' },
  },
  {
    path: '/waf',
    name: 'Waf',
    component: () => import('@/views/WafView.vue'),
    meta: { title: 'nav.waf', icon: 'oi-shield-check', group: 'ops' as MenuGroup, keepAlive: true, weight: 39, keywords: ['waf', '防火墙', '防护', 'web application firewall', 'security'], perm: 'waf:read' },
  },
  {
    path: '/inspection',
    name: 'Inspection',
    component: () => import('@/views/InspectionView.vue'),
    // S-17：安装后系统体检——内核信息 + 风险 / 漏洞分析（toolbox 域，viewer 可读）
    meta: { title: 'nav.inspection', icon: 'oi-wave-pulse', group: 'ops' as MenuGroup, keepAlive: true, weight: 38, keywords: ['inspection', '安全体检', '体检', '内核', 'kernel', '漏洞', 'vulnerability', 'security', 'audit'], perm: 'toolbox:read' },
  },
  {
    path: '/toolbox',
    name: 'Toolbox',
    component: () => import('@/views/ToolboxView.vue'),
    // 系统工具箱：常用运维工具入口 + 主机详情（toolbox:read，viewer 可读）
    meta: { title: 'nav.toolbox', icon: 'oi-briefcase', group: 'ops' as MenuGroup, keepAlive: true, weight: 36, keywords: ['toolbox', '工具箱', '主机详情', 'host', 'hostname', 'dns', 'hosts', 'timezone', '时区', '网卡', 'network', 'disk', '磁盘'], perm: 'toolbox:read' },
  },
  {
    path: '/processes',
    name: 'Processes',
    component: () => import('@/views/ProcessesView.vue'),
    // S-12：系统工具箱 · 进程管理——列表 toolbox:read，结束进程 toolbox:execute
    meta: { title: 'nav.processes', icon: 'oi-microchip', group: 'ops' as MenuGroup, keepAlive: true, weight: 37, keywords: ['processes', 'process', '进程', 'kill', 'top', 'ps', 'pid', '结束进程', 'toolbox', '工具箱'], perm: 'toolbox:read' },
  },
  {
    path: '/runtimes',
    name: 'Runtimes',
    component: () => import('@/views/RuntimeView.vue'),
    meta: { title: 'nav.runtimes', icon: 'oi-layers', group: 'web' as MenuGroup, keepAlive: true, viewName: 'RuntimeView', weight: 30, keywords: ['runtimes', '运行环境', 'php', 'java', 'node', 'python', 'go', 'runtime'], perm: 'runtime:read' },
  },
  {
    path: '/projects',
    name: 'Projects',
    component: () => import('@/views/ProjectView.vue'),
    meta: { title: 'nav.projects', icon: 'oi-folder', group: 'web' as MenuGroup, keepAlive: true, viewName: 'ProjectView', weight: 28, keywords: ['projects', '项目', '部署', 'deploy', 'project'], perm: 'project:read' },
  },
  {
    path: '/web-servers',
    name: 'WebServers',
    component: () => import('@/views/WebServersView.vue'),
    meta: { title: 'nav.webServers', icon: 'oi-server', group: 'web' as MenuGroup, keepAlive: true, weight: 25, keywords: ['web-servers', 'Web服务器', 'nginx', '服务器'], perm: 'web_server:read' },
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: () => import('@/views/TerminalView.vue'),
    meta: { title: 'nav.terminal', icon: 'oi-terminal', group: 'ops' as MenuGroup, weight: 55, keywords: ['terminal', '终端', '命令行', 'ssh'], perm: 'terminal:execute' },
  },
  {
    // Agent Edge 远程交互式终端（不进菜单：从节点列表进入；后端另校验 node:execute）
    path: '/nodes/:id(\\d+)/terminal',
    name: 'NodeTerminal',
    component: () => import('@/views/NodeTerminalView.vue'),
    meta: { title: 'node.terminal', perm: 'terminal:execute' },
  },
  {
    path: '/health',
    name: 'Health',
    component: () => import('@/views/HealthView.vue'),
    meta: { title: 'nav.health', icon: 'oi-wave-pulse', group: 'ops' as MenuGroup, keepAlive: true, weight: 45, keywords: ['health', '健康', '状态', 'status'], perm: 'node:read' },
  },
  {
    path: '/backups',
    name: 'Backups',
    component: () => import('@/views/BackupView.vue'),
    meta: { title: 'nav.backups', icon: 'oi-save', group: 'storage' as MenuGroup, keepAlive: true, viewName: 'BackupView', weight: 35, keywords: ['backups', '备份', '恢复', 'backup'], perm: 'backup:read' },
  },
  {
    path: '/scheduled-tasks',
    name: 'ScheduledTasks',
    component: () => import('@/views/ScheduledTasksView.vue'),
    meta: { title: 'nav.scheduledTasks', icon: 'oi-clock', group: 'ops' as MenuGroup, keepAlive: true, weight: 35, keywords: ['scheduled-tasks', '定时任务', '计划任务', 'cron'], perm: 'scheduled_task:read' },
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/TasksView.vue'),
    meta: { title: 'nav.tasks', icon: 'oi-task', group: 'ops' as MenuGroup, keepAlive: true, weight: 32, keywords: ['tasks', '任务', '进度', 'task', '进度'], perm: 'task:read' },
  },
]

/**
 * keep-alive include 列表（UX-1：多页签移除后由路由表静态推导，不再依赖「已打开页签」）。
 *
 * 组件名 = `meta.viewName ?? name + "View"`——`<script setup>` SFC 的组件名由文件名推断，
 * 三处文件名与路由名不一致的视图（Runtime/Project/Backup）经 `viewName` 显式对齐；
 * 旧实现按 path 拼接（`/runtimes` → `RuntimesView`）导致这三页的 keepAlive 从未生效。
 */
export const keepAliveComponentNames: string[] = menuRoutes
  .filter((r) => r.meta.keepAlive === true)
  .map((r) => (r.meta as { viewName?: string }).viewName ?? `${String(r.name)}View`)

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
    // 面板强制两步验证而当前用户未绑定：独立页（不进 Layout，避免其余接口 403 刷屏）
    {
      path: '/two-factor-setup',
      name: 'TwoFactorSetup',
      component: () => import('@/views/TwoFactorSetupView.vue'),
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

/** 无权限时统一的落地页：被重定向目标自身**不得**再带权限判定（否则形成死循环） */
const DENY_REDIRECT = '/dashboard'

/**
 * 路由守卫（T0-1 拆分）：**登录态 + 初始化状态 + 页面级 RBAC** 三道判定。
 *
 * 独立成具名导出（而非匿名闭包）以便单测直接驱动，避免为测守卫而拼装整棵应用。
 */
export async function routerGuard(to: RouteLocationNormalized): Promise<boolean | RouteLocationRaw> {
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

  return checkPagePermission(to)
}

/**
 * 页面级 RBAC（T0-1，Doc/23 §3.1 / 问题 P0-2）。
 *
 * 此前守卫只查登录态，`isMenuHiddenForRole` 又只作用于侧栏——viewer 直接输入
 * `/users`、`/settings` 的 URL 即可进入页面（API 虽会 403，但页面框架照常渲染，
 * 构成观感越权，且 ⌘K 命令面板照旧列出无权限入口）。
 *
 * 判定与侧栏 / ⌘K 共用 `hasRoutePermission`（同源，杜绝「菜单可见但守卫拒绝」的死路）；
 * 拒绝时重定向 `/dashboard` 并投递一次提示（重定向立即发生，消息经 `notice` 通道
 * 排队投递，保证刷新场景也不丢）。
 *
 * 注：**这不是安全边界**——真正的拦截在服务端 RBAC（Doc/07 §3，默认拒绝）。
 */
export function checkPagePermission(to: RouteLocationNormalized): boolean | RouteLocationRaw {
  const auth = useAuthStore()
  if (hasRoutePermission(auth.role, to.meta)) return true

  emitNotice({ key: 'common.noPermission', severity: 'warn' })
  // 已在目标页（重定向目标自身）时放行，兜底防自环
  return to.path === DENY_REDIRECT ? true : DENY_REDIRECT
}

router.beforeEach(routerGuard)

// S-15：成功进入菜单页后记入 ⌘K「最近访问」（守卫拒绝/重定向的目标不会走到 afterEach 的 to）
router.afterEach((to, _from, failure) => {
  if (failure || !to.meta?.group || typeof to.name !== 'string') return
  recordRecentRoute(useAuthStore().username, to.name)
})

export default router
