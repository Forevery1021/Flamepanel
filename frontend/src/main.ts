import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'

import App from './App.vue'
import router from './router'
import { createPinia } from 'pinia'
import { useTheme } from '@/composables/useTheme'
import { i18n, getLocale } from '@/i18n'

import './style.css'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(router)
app.use(i18n)

// Element Plus locale syncs with i18n
const elLocale = getLocale() === 'en-US' ? en : zhCn
app.use(ElementPlus, { locale: elLocale })

// Apply saved theme before mounting to prevent flash
const { initTheme } = useTheme()
initTheme().then(() => {
  app.mount('#app')
})