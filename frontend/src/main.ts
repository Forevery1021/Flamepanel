import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { createQueryClient } from './composables/useQueryCache'
import OpenVue from 'openvue/config'
import ToastService from 'openvue/toastservice'
import ConfirmationService from 'openvue/confirmationservice'
import ConfirmDialog from 'openvue/confirmdialog'
import Tooltip from 'openvue/tooltip'
import '@openvue/openicons/openicons.css'
import '@fontsource-variable/geist'
import '@fontsource/jetbrains-mono/400.css'
import '@unocss/reset/tailwind.css'
import 'uno.css'
import App from './App.vue'
import router from './router'
import { i18n, preloadSavedLocale } from './locales'
import { applyStoredTheme } from './utils/theme'
import { useThemeStore } from './stores/theme'
import { permission } from './directives/permission'
import { reportError } from './utils/monitor'
import flamePreset from './theme/flame-preset'
import './theme/tokens.css'
import './theme/glass.css'
import './style.css'

applyStoredTheme()

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router)
app.use(VueQueryPlugin, { queryClient: createQueryClient() })
app.use(i18n)
// 启动时异步加载保存的非默认语言包（不阻塞首屏）
preloadSavedLocale()
app.use(OpenVue, {
  theme: {
    preset: flamePreset,
    options: {
      darkModeSelector: '.dark',
      cssLayer: false,
    },
  },
})
app.use(ToastService)
app.use(ConfirmationService)
app.component('ConfirmDialog', ConfirmDialog)
app.directive('tooltip', Tooltip)
app.directive('permission', permission)

// 应用主题 store（令牌/定制），需在 pinia 就绪后执行
useThemeStore().apply()

// P7 可靠性：全局渲染错误捕获（结构化日志上报，预留 Sentry 接入点）
app.config.errorHandler = (err, _instance, info) => {
  reportError(err, { source: 'render', context: info })
}

app.mount('#app')
