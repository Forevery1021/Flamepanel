<template>
  <el-menu :default-active="route.path" router class="sidebar">
    <el-menu-item index="/dashboard">
      <el-icon><Monitor /></el-icon>
      <span>{{ t('nav.dashboard') }}</span>
    </el-menu-item>

    <el-sub-menu index="sys">
      <template #title>
        <el-icon><Setting /></el-icon>
        <span>{{ t('nav.system') }}</span>
      </template>
      <el-menu-item index="/users">
        <el-icon><User /></el-icon>
        <span>{{ t('nav.users') }}</span>
      </el-menu-item>
      <el-menu-item index="/nodes">
        <el-icon><Monitor /></el-icon>
        <span>{{ t('nav.nodes') }}</span>
      </el-menu-item>
      <el-menu-item index="/health">
        <el-icon><InfoFilled /></el-icon>
        <span>{{ t('nav.health') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <el-sub-menu index="resource">
      <template #title>
        <el-icon><Folder /></el-icon>
        <span>{{ t('nav.resource') }}</span>
      </template>
      <el-menu-item index="/files">
        <el-icon><Folder /></el-icon>
        <span>{{ t('nav.files') }}</span>
      </el-menu-item>
      <el-menu-item index="/databases">
        <el-icon><Collection /></el-icon>
        <span>{{ t('nav.databases') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <el-sub-menu index="network">
      <template #title>
        <el-icon><Link /></el-icon>
        <span>{{ t('nav.network') }}</span>
      </template>
      <el-menu-item index="/websites">
        <el-icon><Link /></el-icon>
        <span>{{ t('nav.websites') }}</span>
      </el-menu-item>
      <el-menu-item index="/web-servers">
        <el-icon><Setting /></el-icon>
        <span>{{ t('nav.webServers') }}</span>
      </el-menu-item>
      <el-menu-item index="/docker">
        <el-icon><Ship /></el-icon>
        <span>{{ t('nav.docker') }}</span>
      </el-menu-item>
      <el-menu-item index="/firewall">
        <el-icon><Lock /></el-icon>
        <span>{{ t('nav.firewall') }}</span>
      </el-menu-item>
      <el-menu-item index="/terminal">
        <el-icon><Monitor /></el-icon>
        <span>{{ t('nav.terminal') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <el-menu-item index="/plugins">
      <el-icon><Setting /></el-icon>
      <span>{{ t('nav.plugins') }}</span>
    </el-menu-item>

    <el-sub-menu index="logs">
      <template #title>
        <el-icon><Document /></el-icon>
        <span>{{ t('nav.logs') }}</span>
      </template>
      <el-menu-item index="/operation-logs">
        <el-icon><Document /></el-icon>
        <span>{{ t('nav.operationLogs') }}</span>
      </el-menu-item>
      <el-menu-item index="/system-logs">
        <el-icon><List /></el-icon>
        <span>{{ t('nav.systemLogs') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <el-menu-item index="/settings">
      <el-icon><Tools /></el-icon>
      <span>{{ t('nav.settings') }}</span>
    </el-menu-item>
  </el-menu>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  Monitor,
  User,
  Link,
  Ship,
  Setting,
  Document,
  List,
  Tools,
  InfoFilled,
  Folder,
  Collection,
  Lock,
} from '@element-plus/icons-vue'

const { t } = useI18n()
const route = useRoute()
</script>

<style scoped>
.sidebar {
  width: 220px;
  height: 100vh;
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  background: var(--bg-sidebar);
  --el-menu-bg-color: var(--bg-sidebar);
  --el-menu-text-color: #94a3b8;
  --el-menu-hover-bg-color: rgb(255 255 255 / 0.06);
  --el-menu-active-color: #fff;
  --el-menu-item-height: 44px;
  --el-menu-sub-item-height: 38px;
  --el-menu-base-level-padding: 18px;
}
.sidebar .el-menu {
  border-right: none;
}
.sidebar .el-menu-item:hover,
.sidebar .el-sub-menu__title:hover {
  background: rgb(255 255 255 / 0.06);
}
.sidebar .el-menu-item.is-active {
  background: var(--el-color-primary);
  border-radius: var(--radius-sm);
  margin: 2px var(--space-2);
  width: calc(100% - var(--space-4));
}
.sidebar .el-menu-item,
.sidebar .el-sub-menu__title {
  border-radius: var(--radius-sm);
  margin: 2px var(--space-2);
  width: calc(100% - var(--space-4));
}
.dark .sidebar {
  border-color: var(--border-color);
}
</style>
