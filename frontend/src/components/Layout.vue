<template>
  <div class="layout">
    <Sidebar />
    <div class="layout-right">
      <TopBar />
      <main class="main">
        <router-view />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'
</script>

<style scoped>
.layout { display: flex; height: 100vh; overflow: hidden; }
.layout-right { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.main { flex: 1; padding: 20px; overflow: auto; background: #f5f7fa; }
.dark .main { background: #141414; }
</style>
