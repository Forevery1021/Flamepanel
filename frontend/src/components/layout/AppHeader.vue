<template>
  <header class="app-header glass">
    <div class="header-left">
      <button
        v-tooltip="collapsed ? t('nav.expandSidebar') : t('nav.toggleSidebar')"
        class="icon-btn"
        :aria-label="t('nav.toggleSidebar')"
        @click="isMobile ? $emit('open-mobile') : $emit('toggle-collapse')"
      >
        <i
          class="oi"
          :class="isMobile ? 'oi-bars' : collapsed ? 'oi-chevron-right' : 'oi-chevron-left'"
        />
      </button>

      <Breadcrumb :model="breadcrumbs" class="header-breadcrumb" />

      <button
        class="search-trigger"
        data-tour="search"
        :aria-label="t('topbar.searchPlaceholder')"
        @click="paletteOpen = true"
      >
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

      <!-- S-06：四个徽标来自同一个 GET /api/overview/badges（30 s），分组为 null 即隐藏 -->
      <div class="live-status" data-tour="status" data-testid="overview-badges">
        <button
          v-tooltip="panelOk ? t('topbar.panelOnline') : t('topbar.panelOffline')"
          class="status-item"
          :aria-label="t('topbar.panelStatus')"
          @click="router.push('/health')"
        >
          <span class="dot" :class="panelOk ? 'green' : 'red'" />
        </button>
        <button
          v-if="nodes"
          v-tooltip="t('topbar.onlineNodes')"
          class="status-item"
          data-testid="badge-nodes"
          :aria-label="t('topbar.onlineNodes')"
          @click="router.push('/nodes')"
        >
          <i class="oi oi-sitemap" />
          <span>{{ nodes.online }}</span>
        </button>
        <button
          v-if="containers"
          v-tooltip="t('topbar.runningContainers')"
          class="status-item"
          data-testid="badge-containers"
          :aria-label="t('topbar.runningContainers')"
          @click="router.push('/docker')"
        >
          <i class="oi oi-database" />
          <span>{{ containers.running }}</span>
        </button>
        <button
          v-if="activeTasks"
          v-tooltip="t('topbar.runningTasks')"
          class="status-item status-item--busy"
          data-testid="badge-tasks"
          :aria-label="t('topbar.runningTasks')"
          @click="router.push('/tasks')"
        >
          <i class="oi oi-spinner fp-spin" />
          <span>{{ activeTasks }}</span>
        </button>
      </div>

      <button
        v-tooltip="t('topbar.notifications')"
        class="icon-btn notify-btn"
        data-testid="notify-toggle"
        :aria-label="t('topbar.notifications')"
        @click="notifyOpen = !notifyOpen"
      >
        <i class="oi oi-bell" />
        <span v-if="alertTotal" class="notify-badge" data-testid="alert-count">{{
          alertTotal > 99 ? '99+' : alertTotal
        }}</span>
      </button>
      <Popover :visible="notifyOpen" class="notify-panel" @hide="notifyOpen = false">
        <div class="notify-header">
          <span class="font-semibold">{{ t('topbar.alerts') }}</span>
          <Button text size="small" @click="router.push('/operation-logs')">
            {{ t('topbar.viewAll') }}
          </Button>
        </div>
        <div v-if="alertItems.length" class="notify-list">
          <button
            v-for="a in alertItems"
            :key="a.key"
            class="notify-item notify-item--alert"
            type="button"
            @click="goAlert(a.to)"
          >
            <i class="oi" :class="a.icon" />
            <span class="notify-msg">{{ t(`topbar.alert.${a.key}`, { n: a.count }) }}</span>
            <FpTag severity="warning" :value="String(a.count)" />
          </button>
        </div>
        <div v-else class="notify-empty notify-empty--ok">
          <i class="oi oi-check-circle" />
          {{ t('topbar.noAlerts') }}
        </div>
        <div class="notify-subheader">{{ t('topbar.recentActivity') }}</div>
        <div v-if="notifications.length" class="notify-list">
          <div v-for="n in notifications" :key="n.id" class="notify-item">
            <FpTag
              :severity="n.action.startsWith('LOGIN') ? 'warning' : 'info'"
              :value="n.action"
            />
            <span class="notify-msg">{{ n.target || '' }}</span>
            <span class="notify-time">{{ shortTime(n.created_at) }}</span>
          </div>
        </div>
        <div v-else class="notify-empty">{{ t('common.noData') }}</div>
      </Popover>

      <button
        v-tooltip="t('theme.toggle')"
        class="icon-btn"
        data-tour="theme"
        :aria-label="t('theme.toggle')"
        @click="toggleTheme"
      >
        <i class="oi" :class="themeStore.isDark ? 'oi-sun' : 'oi-moon'" />
      </button>

      <button
        class="user-dropdown"
        data-tour="user"
        :aria-label="t('topbar.userMenu')"
        @click="userMenu?.toggle($event)"
      >
        <span class="user-avatar">{{ auth.username?.charAt(0).toUpperCase() }}</span>
        <span class="user-name">{{ auth.username }}</span>
        <i class="oi oi-angle-down user-arrow" />
      </button>
      <TieredMenu ref="userMenu" :model="userItems" popup />
    </div>

    <CommandPalette v-model="paletteOpen" />
  </header>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Breadcrumb from 'openvue/breadcrumb'
import Popover from 'openvue/popover'
import Button from 'openvue/button'
import TieredMenu from 'openvue/tieredmenu'
import type { MenuItem } from 'openvue/menuitem'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import { useThemeStore } from '@/stores/theme'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { InstalledApp } from '@/api/appStore'
import { listOperationLogs } from '@/api/logs'
import FpTag from '@/components/ui/FpTag.vue'
import CommandPalette from './CommandPalette.vue'
import { useOnboardingStore } from '@/stores/onboarding'
import { useOverviewBadges } from '@/composables/useOverviewBadges'

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
const onboarding = useOnboardingStore()

// ── 面包屑 ──
const breadcrumbs = computed(() => {
  const title = route.meta?.title
  if (!title) return []
  return [{ label: t(title) }]
})

// ── 实时状态（S-06：单接口聚合，替代原三路 10 s 轮询） ──
const { panelOk, nodes, containers, activeTasks, alertTotal, alertItems } = useOverviewBadges()

// ── 通知中心：告警来自聚合徽标；最近操作日志只在面板打开时拉取，不再周期轮询 ──
const notifyOpen = ref(false)
const notifications = ref<
  Array<{ id: number; action: string; target: string | null; created_at: string }>
>([])

watch(notifyOpen, (open) => {
  if (open) refreshNotifications()
})

function goAlert(to: string) {
  notifyOpen.value = false
  router.push(to)
}

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
  if (app.access_url) window.open(app.access_url, '_blank', 'noopener')
}

// ── 语言 / 用户菜单 ──
const userMenu = ref<InstanceType<typeof TieredMenu>>()

const langItems = computed<MenuItem[]>(() =>
  ['zh-CN', 'en-US', 'ja-JP'].map((lang) => ({
    label: lang === 'zh-CN' ? '简体中文' : lang === 'en-US' ? 'English' : '日本語',
    icon: locale.value === lang ? 'oi oi-check' : undefined,
    command: () => setLanguage(lang),
  })),
)

/** 用户菜单：设置 / 语言（子菜单，从顶栏独立图标合并进来以减负）/ 退出 */
const userItems = computed<MenuItem[]>(() => [
  {
    label: t('nav.settings'),
    icon: 'oi oi-cog',
    command: () => router.push('/settings'),
  },
  {
    label: t('topbar.language'),
    icon: 'oi oi-language',
    items: langItems.value,
  },
  {
    label: t('onboarding.replay'),
    icon: 'oi oi-compass',
    command: () => onboarding.start(),
  },
  { separator: true },
  {
    label: t('common.logout') || '退出登录',
    icon: 'oi oi-power-off',
    command: () => {
      auth.logout()
      router.push('/login')
    },
  },
])

function toggleTheme() {
  themeStore.toggleDark()
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
  refreshInstalled()
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
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

.header-breadcrumb {
  min-width: 0;
  flex: 0 1 auto;
  overflow: hidden;
}
.header-breadcrumb :deep(.p-breadcrumb) {
  background: transparent;
  border: none;
  padding: 0;
}
/* 面包屑单行省略：窄窗口下旧版会逐字换行成竖排 */
.header-breadcrumb :deep(.p-breadcrumb-list) {
  flex-wrap: nowrap;
  white-space: nowrap;
}
.header-breadcrumb :deep(.p-breadcrumb-item-label) {
  overflow: hidden;
  text-overflow: ellipsis;
}

.search-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  width: clamp(160px, 22vw, 260px);
  flex: 0 1 auto;
  min-width: 0;
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
.status-item--busy {
  color: var(--fp-brand);
}
.fp-spin {
  animation: fp-rotate 0.8s linear infinite;
}
@keyframes fp-rotate {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .fp-spin {
    animation: none;
  }
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
.notify-item--alert {
  width: 100%;
  border-left: none;
  border-right: none;
  border-top: none;
  background: transparent;
  color: var(--fp-text-primary);
  cursor: pointer;
  text-align: left;
  font-family: var(--fp-font-sans);
  border-radius: var(--fp-radius-sm);
}
.notify-item--alert:hover {
  background: var(--fp-bg-hover);
}
.notify-item--alert .oi {
  color: var(--fp-warning);
  font-size: 14px;
}
.notify-subheader {
  margin-top: var(--fp-space-3);
  padding: var(--fp-space-1) 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--fp-text-muted);
}
.notify-empty {
  padding: var(--fp-space-4);
  text-align: center;
  color: var(--fp-text-muted);
  font-size: 13px;
}
.notify-empty--ok {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  color: var(--fp-success);
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

/* 平板：让出空间给搜索框，状态徽标只保留圆点 */
@media (max-width: 1100px) {
  .quick-apps,
  .user-name,
  .user-arrow {
    display: none;
  }
  .search-kbd {
    display: none;
  }
}
@media (max-width: 900px) {
  .header-breadcrumb {
    display: none;
  }
}
/* 手机：搜索收成图标按钮（旧版 140px 宽框里塞图标 + kbd，二者重叠） */
@media (max-width: 767px) {
  .app-header {
    padding: 0 var(--fp-space-2);
  }
  .live-status {
    display: none;
  }
  .search-trigger {
    width: 32px;
    padding: 0;
    justify-content: center;
    background: transparent;
    border-color: transparent;
  }
  .search-text {
    display: none;
  }
}
</style>
