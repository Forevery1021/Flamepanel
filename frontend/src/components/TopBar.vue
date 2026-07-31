<template>
  <header class="topbar">
    <div class="topbar-left">
      <span class="topbar-logo">FlamePanel</span>
      <span class="topbar-divider" />
      <span class="topbar-title">{{ t(`nav.${routeName}`) || routeName }}</span>
    </div>
    <div class="topbar-right">
      <el-dropdown trigger="click" @command="handleLangChange">
        <el-button text circle>
          <el-icon size="18"><ChatDotRound /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="zh-CN" :disabled="locale === 'zh-CN'"
              >简体中文</el-dropdown-item
            >
            <el-dropdown-item command="en-US" :disabled="locale === 'en-US'"
              >English</el-dropdown-item
            >
            <el-dropdown-item command="ja-JP" :disabled="locale === 'ja-JP'"
              >日本語</el-dropdown-item
            >
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <el-tooltip :content="t('theme.toggle')" placement="bottom">
        <el-button text circle @click="toggleTheme">
          <el-icon size="18"><Moon v-if="isDark" /><Sunny v-else /></el-icon>
        </el-button>
      </el-tooltip>
      <el-tooltip :content="t('health.title')" placement="bottom">
        <el-button text circle @click="$router.push('/health')">
          <el-icon size="18"><InfoFilled /></el-icon>
        </el-button>
      </el-tooltip>
      <el-dropdown trigger="click" @command="handleCommand">
        <span class="user-dropdown">
          <el-icon><UserFilled /></el-icon>
          <span>{{ auth.username }}</span>
          <el-icon><ArrowDown /></el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="settings">
              <el-icon><Tools /></el-icon> {{ t('nav.settings') }}
            </el-dropdown-item>
            <el-dropdown-item command="logout" divided>
              <el-icon><SwitchButton /></el-icon> {{ t('common.logout') || '退出登录' }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import {
  UserFilled,
  Tools,
  SwitchButton,
  InfoFilled,
  ArrowDown,
  Moon,
  Sunny,
  ChatDotRound,
} from '@element-plus/icons-vue'

const { t, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const auth = useAuthStore()

const routeName = computed(() => (route.name as string) || '')

const isDark = computed(() => document.documentElement.classList.contains('dark'))

function toggleTheme() {
  const html = document.documentElement
  html.classList.toggle('dark')
  localStorage.setItem('flame-theme', html.classList.contains('dark') ? 'dark' : 'light')
}

function handleLangChange(lang: string) {
  setLanguage(lang)
}

function handleCommand(cmd: string) {
  if (cmd === 'logout') {
    auth.logout()
    router.push('/login')
  } else if (cmd === 'settings') {
    router.push('/settings')
  }
}
</script>

<style scoped>
.topbar {
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}
.topbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}
.topbar-logo {
  font-size: 18px;
  font-weight: 700;
  color: var(--brand);
  letter-spacing: 1px;
}
.topbar-divider {
  width: 1px;
  height: 20px;
  background: var(--border-strong);
}
.topbar-title {
  font-size: 15px;
  color: var(--text-primary);
  font-weight: 500;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.user-dropdown {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: var(--radius-md);
  font-size: 14px;
  color: var(--text-secondary);
  transition:
    background-color var(--transition-fast),
    color var(--transition-fast);
}
.user-dropdown:hover {
  background: var(--bg-hover);
}
.dark .topbar {
  background: var(--bg-secondary);
  border-color: var(--border-color);
}
.dark .topbar-title {
  color: var(--text-primary);
}
.dark .user-dropdown {
  color: var(--text-secondary);
}
.dark .user-dropdown:hover {
  background: var(--bg-hover);
}
</style>
