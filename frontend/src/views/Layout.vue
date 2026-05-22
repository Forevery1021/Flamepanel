<script setup lang="ts">
import Sidebar from '@/components/Sidebar.vue'
</script>

<template>
  <div class="layout">
    <Sidebar />
    <div class="main-content">
      <router-view />
    </div>
  </div>
</template>