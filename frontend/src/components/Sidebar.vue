<template>
  <el-menu :default-active="route.path" router class="sidebar">
    <el-menu-item index="/dashboard">
      <el-icon><Monitor /></el-icon>
      <span>仪表盘</span>
    </el-menu-item>

    <el-sub-menu index="sys">
      <template #title>
        <el-icon><Setting /></el-icon>
        <span>系统管理</span>
      </template>
      <el-menu-item index="/users">
        <el-icon><User /></el-icon>
        <span>用户管理</span>
      </el-menu-item>
      <el-menu-item index="/nodes">
        <el-icon><Monitor /></el-icon>
        <span>节点管理</span>
      </el-menu-item>
      <el-menu-item index="/health">
        <el-icon><InfoFilled /></el-icon>
        <span>系统健康</span>
      </el-menu-item>
    </el-sub-menu>

    <el-sub-menu index="resource">
      <template #title>
        <el-icon><Folder /></el-icon>
        <span>资源管理</span>
      </template>
      <el-menu-item index="/files">
        <el-icon><Folder /></el-icon>
        <span>文件管理</span>
      </el-menu-item>
      <el-menu-item index="/databases">
        <el-icon><Collection /></el-icon>
        <span>数据库管理</span>
      </el-menu-item>
    </el-sub-menu>

    <el-sub-menu index="network">
      <template #title>
        <el-icon><Link /></el-icon>
        <span>网络服务</span>
      </template>
      <el-menu-item index="/websites">
        <el-icon><Link /></el-icon>
        <span>网站管理</span>
      </el-menu-item>
      <el-menu-item index="/docker">
        <el-icon><Ship /></el-icon>
        <span>Docker</span>
      </el-menu-item>
      <el-menu-item index="/firewall">
        <el-icon><Lock /></el-icon>
        <span>防火墙</span>
      </el-menu-item>
      <el-menu-item index="/terminal">
        <el-icon><Monitor /></el-icon>
        <span>Web 终端</span>
      </el-menu-item>
    </el-sub-menu>

    <el-menu-item index="/plugins">
      <el-icon><Setting /></el-icon>
      <span>插件管理</span>
    </el-menu-item>

    <el-sub-menu index="logs">
      <template #title>
        <el-icon><Document /></el-icon>
        <span>运维日志</span>
      </template>
      <el-menu-item index="/operation-logs">
        <el-icon><Document /></el-icon>
        <span>操作日志</span>
      </el-menu-item>
      <el-menu-item index="/system-logs">
        <el-icon><List /></el-icon>
        <span>系统日志</span>
      </el-menu-item>
    </el-sub-menu>

    <el-menu-item index="/settings">
      <el-icon><Tools /></el-icon>
      <span>面板设置</span>
    </el-menu-item>
  </el-menu>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { Monitor, User, Link, Ship, Setting, Document, List, Tools, InfoFilled, Folder, Collection, Lock, Flag } from '@element-plus/icons-vue'

const route = useRoute()
</script>

<style scoped>
.sidebar { width: 220px; height: 100vh; border-right: 1px solid #e4e7ed; overflow-y: auto; background: #fff; }
.dark .sidebar { background: #1d1e1f; border-color: #333; }
</style>
