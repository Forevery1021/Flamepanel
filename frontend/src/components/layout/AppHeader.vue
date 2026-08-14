<template>
  <header class="app-header glass">
    <div class="header-left">
      <button
        v-tooltip="collapsed ? t('nav.expandSidebar') : t('nav.toggleSidebar')"
        class="icon-btn"
        :aria-label="t('nav.toggleSidebar')"
        @click="isMobile ? $emit('open-mobile') : $emit('toggle-collapse')"      >
        <i class="oi" :class="isMobile ? 'oi-bars' : collapsed ? 'oi-chevron-right' : 'oi-chevron-left'" />
      </button>

      <Breadcrumb :model="breadcrumbs" class="header-breadcrumb" />

      <button class="search-trigger" :aria-label="t('topbar.searchPlaceholder')" @click="paletteOpen = true">
        <i class="oi oi-search" />
        <span class="search-text">{{ t('topbar.searchPlaceholder') }}</span>
        <kbd class="search-kbd">Ctrl K</kbd>
      </button>
    </div>

    <div class="header-right">
      <div v-if="quickApps.length" class="quick-apps">
        <button
          v-for="app in quickApps"
          :key="app.id"
          v-tooltip="app.name"
          class="quick-app"
          :aria-label="t('topbar.app') + ': ' + app.name"
          @click="openApp(app)"
        >
          <i class="oi oi-box" />
        </button>
      </div>

      <div class="live-status">
        <button v-tooltip="t('topbar.panelStatus')" class="status-item" :aria-label="t('topbar.panelStatus')" @click="router.push('/health')">
          <span class="dot" :class="panelOk ? 'green' : 'red'" />
        </button>
        <button v-tooltip="t('topbar.onlineNodes')" class="status-item" :aria-label="t('topbar.onlineNodes')" @click="router.push('/nodes')">
          <i class="oi oi-sitemap" />
          <span>{{ onlineNodes }}</span>
        </button>
        <button v-tooltip="t('topbar.runningContainers')" class="status-item" :aria-label="t('topbar.runningContainers')" @click="router.push('/docker')">
          <i class="oi oi-database" />
          <span>{{ runningContainers }}</span>
        </button>
      </div>

      <button v-tooltip="t('topbar.notifications')" class="icon-btn notify-btn" :aria-label="t('topbar.notifications')" @click="notifyOpen = !notifyOpen">
        <i class="oi oi-bell" />
        <span v-if="notifyCount" class="notify-badge">{{ notifyCount }}</span>
      </button>
      <Popover :visible="notifyOpen" class="notify-panel" @hide="notifyOpen = false">
        <div class="notify-header">
          <span class="font-semibold">{{ t('topbar.notifications') }}</span>
          <Button text size="small" @click="router.push('/operation-logs')">
            {{ t('topbar.viewAll') }}
          </Button>
        </div>
        <div v-if="notifications.length" class="notify-list">
          <div v-for="n in notifications" :key="n.id" class="notify-item">
            <FpTag :severity="n.action.startsWith('LOGIN') ? 'warning' : 'info'" :value="n.action" />
            <span class="notify-msg">{{ n.target || '' }}</span>
            <span class="notify-time">{{ shortTime(n.created_at) }}</span>
          </div>
        </div>
        <div v-else class="notify-empty">{{ t('common.noData') }}</div>
      </Popover>

      <button v-tooltip="t('topbar.language')" class="icon-btn" :aria-label="t('topbar.language')" @click="langMenu?.toggle($event)">
        <i class="oi oi-language" />
      </button>
      <Menu ref="langMenu" :model="langItems" popup />

      <button v-tooltip="t('theme.toggle')" class="icon-btn" :aria-label="t('theme.toggle')" @click="toggleTheme">
        <i class="oi" :class="themeStore.mode === 'dark' ? 'oi-sun' : 'oi-moon'" />
      </button>

      <button class="user-dropdown" @click="userMenu?.toggle($event)">
        <span class="user-avatar">{{ auth.username?.charAt(0).toUpperCase() }}</span>
        <span class="user-name">{{ auth.username }}</span>
        <i class="oi oi-angle-down user-arrow" />
      </button>
      <Menu ref="userMenu" :model="userItems" popup />
    </div>

    <CommandPalette v-model="paletteOpen" />
  </header>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Breadcrumb from 'openvue/breadcrumb'
import Popover from 'openvue/popover'
import Button from 'openvue/button'
import Menu from 'openvue/menu'
import type { MenuItem } from 'openvue/menuitem'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import { useThemeStore } from '@/stores/theme'
import { listNodes } from '@/api/nodes'
import { listContainers } from '@/api/docker'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { InstalledApp } from '@/api/appStore'
import { listOperationLogs } from '@/api/logs'
import FpTag from '@/components/ui/FpTag.vue'
import CommandPalette from './CommandPalette.vue'

defineProps<{
  collapsed?: boolean
  isMobile?: boolean
}>()
defineEmits<{
  (e: 'toggle-collapse'): void
  (e: 'open-mobile'): void
}>()

const { t, locale } = useI18n()
const router = useRouter()
const route = useRoute()
const auth = useAuthStore()
const themeStore = useThemeStore()

// ── 面包屑 ──
const breadcrumbs = computed(() => {
  const title = route.meta?.title
  if (!title) return []
  return [{ label: t(title) }]
})

// ── 实时状态 ──
const panelOk = ref(true)
const onlineNodes = ref(0)
const runningContainers = ref(0)
let statusTimer: number | null = null

async function refreshStatus() {
  try {
    const [nodes, containers] = await Promise.all([
      listNodes(1, 200).catch(() => null),
      listContainers().catch(() => null),
    ])
    onlineNodes.value =
      nodes?.data.data.filter((n: { status: string }) => n.status === 'online').length ?? 0
    runningContainers.value =
      containers?.data.filter((c: { status: string }) => c.status === 'running').length ?? 0
    panelOk.value = true
  } catch {
    panelOk.value = false
  }
}

// ── 通知中心 ──
const notifyOpen = ref(false)
const notifications = ref<
  Array<{ id: number; action: string; target: string | null; created_at: string }>
>([])
const notifyCount = computed(() => notifications.value.length)

async function refreshNotifications() {
  try {
    const res = await listOperationLogs(1, 6)
    notifications.value = res.data?.data ?? []
  } catch {
    notifications.value = []
  }
}

// ── 快捷应用 ──
const installedApps = ref<InstalledApp[]>([])
const quickApps = computed(() =>
  [...installedApps.value]
    .sort((a, b) => (b.launch_count ?? 0) - (a.launch_count ?? 0))
    .slice(0, 4),
)

async function refreshInstalled() {
  try {
    const res = await listInstalledApps()
    installedApps.value = Array.isArray(res.data) ? res.data : []
  } catch {
    installedApps.value = []
  }
}

async function openApp(app: InstalledApp) {
  launchApp(app.id).catch(() => {})
  if (app.access_url) window.open(app.access_url, '_blank')
}

// ── 语言 / 用户菜单 ──
const langMenu = ref<InstanceType<typeof Menu>>()
const userMenu = ref<InstanceType<typeof Menu>>()

const langItems = computed<MenuItem[]>(() =>
  ['zh-CN', 'en-US', 'ja-JP'].map((lang) => ({
    label: lang === 'zh-CN' ? '简体中文' : lang === 'en-US' ? 'English' : '日本語',
    icon: locale.value === lang ? 'oi oi-check' : undefined,
    command: () => setLanguage(lang),
  })),
)

const userItems: MenuItem[] = [
  {
    label: computed(() => t('nav.settings')).value,
    icon: 'oi oi-cog',
    command: () => router.push('/settings'),
  },
  { separator: true },
  {
    label: computed(() => t('common.logout') || '退出登录').value,
    icon: 'oi oi-power-off',
    command: () => {
      auth.logout()
      router.push('/login')
    },
  },
]

function toggleTheme() {
  themeStore.setMode(themeStore.mode === 'dark' ? 'light' : 'dark')
}

function shortTime(ts: string) {
  const d = new Date(ts)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

// ── 命令面板 ──
const paletteOpen = ref(false)
function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    paletteOpen.value = !paletteOpen.value
  }
}

onMounted(() => {
  refreshStatus()
  refreshNotifications()
  refreshInstalled()
  statusTimer = window.setInterval(() => {
    refreshStatus()
    refreshNotifications()
  }, 10000)
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  if (statusTimer !== null) clearInterval(statusTimer)
  window.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--fp-header-height);
  padding: 0 var(--fp-space-4);
  border-bottom: 1px solid var(--fp-border);
  flex-shrink: 0;
  gap: var(--fp-space-3);
  position: relative;
  z-index: var(--fp-z-header);
}
.header-left {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  min-width: 0;
  flex: 1;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: var(--fp-text-secondary);
  font-size: 15px;
  cursor: pointer;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast),
    transform 120ms var(--fp-ease-out);
}
.icon-btn:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.icon-btn:active {
  transform: scale(0.94);
}

.header-breadcrumb :deep(.p-breadcrumb) {
  background: transparent;
  border: none;
  padding: 0;
}

.search-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  width: 240px;
  padding: 0 10px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-hover);
  border: 1px solid var(--fp-border);
  color: var(--fp-text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: border-color var(--fp-transition-fast);
}
.search-trigger:hover,
.search-trigger:focus-visible {
  border-color: var(--fp-brand);
}
.search-text {
  flex: 1;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.search-kbd {
  font-family: var(--fp-font-mono);
  font-size: 10px;
  color: var(--fp-text-muted);
  border: 1px solid var(--fp-border-strong);
  border-radius: 4px;
  padding: 1px 5px;
  flex-shrink: 0;
}

.quick-apps {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-right: var(--fp-space-2);
}
.quick-app {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: var(--fp-text-secondary);
  cursor: pointer;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.quick-app:hover {
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
}

.live-status {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-right: var(--fp-space-2);
}
.status-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 13px;
  color: var(--fp-text-secondary);
  cursor: pointer;
  padding: 5px 8px;
  border: none;
  background: transparent;
  border-radius: var(--fp-radius-sm);
  font-family: var(--fp-font-sans);
}
.status-item:hover {
  background: var(--fp-bg-hover);
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
.dot.green {
  background: var(--fp-success);
  box-shadow: 0 0 0 3px var(--fp-success-soft);
}
.dot.red {
  background: var(--fp-danger);
  box-shadow: 0 0 0 3px var(--fp-danger-soft);
}

.notify-btn {
  position: relative;
}
.notify-badge {
  position: absolute;
  top: 1px;
  right: 1px;
  min-width: 14px;
  height: 14px;
  border-radius: 7px;
  background: var(--fp-danger);
  color: #fff;
  font-size: 10px;
  line-height: 14px;
  text-align: center;
  padding: 0 3px;
}
.notify-panel {
  width: 340px;
}
.notify-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: var(--fp-space-2);
  border-bottom: 1px solid var(--fp-border);
  margin-bottom: var(--fp-space-2);
}
.notify-list {
  display: flex;
  flex-direction: column;
}
.notify-item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) 0;
  border-bottom: 1px solid var(--fp-border);
  font-size: 12px;
}
.notify-msg {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--fp-text-secondary);
}
.notify-time {
  color: var(--fp-text-muted);
  flex-shrink: 0;
  font-family: var(--fp-font-mono);
  font-size: 11px;
}
.notify-empty {
  padding: var(--fp-space-4);
  text-align: center;
  color: var(--fp-text-muted);
  font-size: 13px;
}

.user-dropdown {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border: none;
  background: transparent;
  border-radius: var(--fp-radius-md);
  font-family: var(--fp-font-sans);
  color: var(--fp-text-secondary);
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.user-dropdown:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.user-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  background: linear-gradient(135deg, var(--fp-brand), var(--fp-brand-strong));
  color: #fff;
  font-size: 12px;
  font-weight: 700;
}
.user-name {
  font-size: 13.5px;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-arrow {
  font-size: 10px;
}

@media (max-width: 768px) {
  .header-breadcrumb,
  .quick-apps,
  .live-status {
    display: none;
  }
  .search-trigger {
    width: 140px;
  }
}
@media (max-width: 480px) {
  .user-name,
  .user-arrow {
    display: none;
  }
  .search-text {
    display: none;
  }
  .search-trigger {
    width: 34px;
    justify-content: center;
  }
}
</style>
