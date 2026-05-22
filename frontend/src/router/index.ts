import { createRouter, createWebHistory, RouterView } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/LoginView.vue'),
      meta: { requiresAuth: false }
    },
    {
      path: '/',
      // Use RouterView as the layout placeholder to avoid missing Layout.vue
      component: RouterView,
      redirect: '/dashboard',
      meta: { requiresAuth: true },
      children: [
        { path: 'dashboard', name: 'Dashboard', component: () => import('@/views/DashboardView.vue') },
        { path: 'file', name: 'FileManager', component: () => import('@/views/FileManagerView.vue') },
        { path: 'docker', name: 'Docker', component: () => import('@/views/DockerView.vue') },
        { path: 'terminal', name: 'Terminal', component: () => import('@/views/TerminalView.vue') },
      ]
    }
  ]
})

// 路由守卫
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