import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/LoginView.vue'),
      meta: { requiresAuth: false },
    },
    {
      path: '/',
      component: () => import('@/layout/Layout.vue'),
      redirect: '/dashboard',
      meta: { requiresAuth: true },
      children: [
        {
          path: 'dashboard',
          name: 'Dashboard',
          component: () => import('@/views/DashboardView.vue'),
        },
        {
          path: 'file',
          name: 'FileManager',
          component: () => import('@/views/FileManagerView.vue'),
        },
        {
          path: 'docker',
          name: 'Docker',
          component: () => import('@/views/DockerView.vue'),
        },
        {
          path: 'databases',
          name: 'Databases',
          component: () => import('@/views/DatabaseView.vue'),
        },
        {
          path: 'website',
          name: 'Website',
          component: () => import('@/views/WebsiteView.vue'),
        },
        {
          path: 'waf',
          name: 'Waf',
          component: () => import('@/views/WafView.vue'),
        },
        {
          path: 'terminal',
          name: 'Terminal',
          component: () => import('@/views/TerminalView.vue'),
        },
        {
          path: 'users',
          name: 'Users',
          component: () => import('@/views/UsersView.vue'),
        },
        {
          path: 'logs',
          name: 'Logs',
          component: () => import('@/views/LogView.vue'),
        },
        {
          path: 'processes',
          name: 'Processes',
          component: () => import('@/views/ProcessView.vue'),
        },
        {
          path: 'cleanup',
          name: 'Cleanup',
          component: () => import('@/views/CleanupView.vue'),
        },
        {
          path: 'settings',
          name: 'Settings',
          component: () => import('@/views/SettingsView.vue'),
        },
        {
          path: 'cron',
          name: 'Cron',
          component: () => import('@/views/CronView.vue'),
        },
        {
          path: 'appstore',
          name: 'AppStore',
          component: () => import('@/views/AppStoreView.vue'),
        },
      ],
    },
  ],
})

router.beforeEach((to, _, next) => {
  const auth = useAuthStore()
  if (to.meta.requiresAuth && !auth.token) {
    next('/login')
  } else if (to.path === '/login' && auth.token) {
    next('/dashboard')
  } else {
    next()
  }
})

export default router
