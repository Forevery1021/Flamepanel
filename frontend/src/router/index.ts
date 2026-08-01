import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

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
      children: [
        {
          path: 'dashboard',
          name: 'Dashboard',
          component: () => import('@/views/DashboardView.vue'),
        },
        { path: 'users', name: 'Users', component: () => import('@/views/UsersView.vue') },
        { path: 'nodes', name: 'Nodes', component: () => import('@/views/NodesView.vue') },
        { path: 'files', name: 'Files', component: () => import('@/views/FilesView.vue') },
        {
          path: 'databases',
          name: 'Databases',
          component: () => import('@/views/DatabasesView.vue'),
        },
        { path: 'websites', name: 'Websites', component: () => import('@/views/WebsitesView.vue') },
        { path: 'docker', name: 'Docker', component: () => import('@/views/DockerView.vue') },
        { path: 'plugins', name: 'Plugins', component: () => import('@/views/PluginsView.vue') },
        { path: 'app-store', name: 'AppStore', component: () => import('@/views/AppStoreView.vue') },
        {
          path: 'operation-logs',
          name: 'OperationLogs',
          component: () => import('@/views/OperationLogsView.vue'),
        },
        {
          path: 'system-logs',
          name: 'SystemLogs',
          component: () => import('@/views/SystemLogsView.vue'),
        },
        { path: 'settings', name: 'Settings', component: () => import('@/views/SettingsView.vue') },
        { path: 'firewall', name: 'Firewall', component: () => import('@/views/FirewallView.vue') },
        {
          path: 'web-servers',
          name: 'WebServers',
          component: () => import('@/views/WebServersView.vue'),
        },
        { path: 'terminal', name: 'Terminal', component: () => import('@/views/TerminalView.vue') },
        { path: 'health', name: 'Health', component: () => import('@/views/HealthView.vue') },
      ],
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
