<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSystemStore } from '@/stores/system'
import { ElRow, ElCol, ElCard } from 'element-plus'

const system = useSystemStore()
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  await system.fetchSystemInfo()
  loading.value = false
})
</script>

<template>
  <div class="dashboard">
    <h1>系统概览</h1>
    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>CPU 使用率</h3>
            <p class="value">{{ system.cpuUsage.toFixed(1) }}%</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>内存</h3>
            <p class="value">{{ system.memoryUsed }} / {{ system.memoryTotal }} GB</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>系统运行时间</h3>
            <p class="value">{{ system.uptime }}</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>在线容器</h3>
            <p class="value">{{ system.dockerContainers }}</p>
          </div>
        </ElCard>
      </ElCol>
    </ElRow>

    <!-- Docker 容器列表 -->
    <ElCard class="mt-20">
      <template #header>运行中的容器</template>
      <!-- 表格省略，可自行扩展 -->
    </ElCard>
  </div>
</template>

<style scoped>
.stat h3 { margin: 0; font-size: 14px; color: #909399; }
.stat .value { font-size: 28px; font-weight: bold; margin: 10px 0; }
</style>