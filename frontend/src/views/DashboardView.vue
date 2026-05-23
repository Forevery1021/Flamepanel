<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useDashboardStore } from '@/stores/dashboard'
import { useMetricsWebSocket } from '@/composables/useMetricsWebSocket'
import SystemChart from '@/components/SystemChart.vue'
import { ElRow, ElCol, ElCard, ElProgress, ElTable, ElTableColumn, ElTag } from 'element-plus'

const dashboard = useDashboardStore()
const metrics = useMetricsWebSocket()
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  await dashboard.fetchDashboard()
  loading.value = false
  metrics.connect()
})

onUnmounted(() => {
  metrics.disconnect()
})

function formatBytes(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  return `${mb.toFixed(0)} MB`
}

function uptimeDisplay(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  return `${days}d ${hours}h ${mins}m`
}
</script>

<template>
  <div class="dashboard" v-loading="loading">
    <h1 style="margin-top: 0">系统概览</h1>

    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>CPU 使用率</h3>
            <ElProgress
              type="dashboard"
              :percentage="Math.round(dashboard.serverInfo?.cpu_usage ?? 0)"
              :color="(dashboard.serverInfo?.cpu_usage ?? 0) > 80 ? '#f56c6c' : '#409eff'"
            />
            <p class="sub">{{ dashboard.serverInfo?.cpu_cores ?? 0 }} 核心</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>内存使用率</h3>
            <ElProgress
              type="dashboard"
              :percentage="Math.round(dashboard.serverInfo ? (dashboard.serverInfo.memory_used_mb / dashboard.serverInfo.memory_total_mb) * 100 : 0)"
              :color="(dashboard.serverInfo ? (dashboard.serverInfo.memory_used_mb / dashboard.serverInfo.memory_total_mb) * 100 : 0) > 80 ? '#f56c6c' : '#67c23a'"
            />
            <p class="sub">
              {{ formatBytes(dashboard.serverInfo?.memory_used_mb ?? 0) }} /
              {{ formatBytes(dashboard.serverInfo?.memory_total_mb ?? 0) }}
            </p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>磁盘使用率</h3>
            <ElProgress
              type="dashboard"
              :percentage="Math.round(dashboard.serverInfo ? (dashboard.serverInfo.disk_used_gb / dashboard.serverInfo.disk_total_gb) * 100 : 0)"
              :color="(dashboard.serverInfo ? (dashboard.serverInfo.disk_used_gb / dashboard.serverInfo.disk_total_gb) * 100 : 0) > 80 ? '#f56c6c' : '#e6a23c'"
            />
            <p class="sub">
              {{ (dashboard.serverInfo?.disk_used_gb ?? 0).toFixed(0) }} /
              {{ (dashboard.serverInfo?.disk_total_gb ?? 0).toFixed(0) }} GB
            </p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard>
          <div class="stat">
            <h3>系统运行时间</h3>
            <p class="value">{{ uptimeDisplay(dashboard.serverInfo?.uptime_seconds ?? 0) }}</p>
            <p class="sub">{{ dashboard.serverInfo?.network?.hostname || '-' }}</p>
          </div>
        </ElCard>
      </ElCol>
    </ElRow>

    <!-- 概览卡片 -->
    <ElRow :gutter="20" style="margin-top: 20px">
      <ElCol :span="6">
        <ElCard class="overview-card">
          <div class="overview-stat">
            <h3>Docker 容器</h3>
            <p class="big-value">
              <span class="green">{{ dashboard.dockerRunning }}</span>
              /
              <span>{{ dashboard.dockerTotal }}</span>
            </p>
            <p class="sub">运行中 / 总计</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard class="overview-card">
          <div class="overview-stat">
            <h3>网站</h3>
            <p class="big-value">
              <span class="green">{{ dashboard.websitesRunning }}</span>
              /
              <span>{{ dashboard.websitesTotal }}</span>
            </p>
            <p class="sub">运行中 / 总计</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard class="overview-card">
          <div class="overview-stat">
            <h3>WAF 规则</h3>
            <p class="big-value">
              <span class="green">{{ dashboard.wafRulesEnabled }}</span>
              /
              <span>{{ dashboard.wafRulesCount }}</span>
            </p>
            <p class="sub">启用 / 总计</p>
          </div>
        </ElCard>
      </ElCol>
      <ElCol :span="6">
        <ElCard class="overview-card">
          <div class="overview-stat">
            <h3>系统负载</h3>
            <p class="value-text">
              {{ dashboard.serverInfo?.load_average?.one?.toFixed(2) ?? '-' }}
              /
              {{ dashboard.serverInfo?.load_average?.five?.toFixed(2) ?? '-' }}
              /
              {{ dashboard.serverInfo?.load_average?.fifteen?.toFixed(2) ?? '-' }}
            </p>
            <p class="sub">1m / 5m / 15m</p>
          </div>
        </ElCard>
      </ElCol>
    </ElRow>

    <!-- 实时监控图表 -->
    <ElRow :gutter="20" style="margin-top: 20px">
      <ElCol :span="12">
        <ElCard>
          <SystemChart title="CPU 使用率 (%)" :history="metrics.history.value"
            value-key="cpu_usage" color="#409eff" />
        </ElCard>
      </ElCol>
      <ElCol :span="12">
        <ElCard>
          <SystemChart title="内存使用率 (%)" :history="metrics.history.value"
            value-key="memory_usage_percent" color="#67c23a" />
        </ElCard>
      </ElCol>
    </ElRow>

    <ElRow :gutter="20" style="margin-top: 20px">
      <ElCol :span="12">
        <ElCard>
          <SystemChart title="磁盘使用率 (%)" :history="metrics.history.value"
            value-key="disk_usage_percent" color="#e6a23c" />
        </ElCard>
      </ElCol>
      <ElCol :span="12">
        <ElCard>
          <SystemChart title="系统负载" :history="metrics.history.value"
            :value-keys="['load_one', 'load_five', 'load_fifteen']"
            :series-names="['1m', '5m', '15m']"
            :colors="['#409eff', '#67c23a', '#e6a23c']" unit="" />
        </ElCard>
      </ElCol>
    </ElRow>

    <!-- 最近操作日志 -->
    <ElCard style="margin-top: 20px">
      <template #header>
        <span>最近操作日志</span>
      </template>
      <ElTable :data="dashboard.recentLogs" stripe size="small" max-height="300">
        <ElTableColumn prop="username" label="用户" width="120" />
        <ElTableColumn prop="action" label="操作" width="180">
          <template #default="{ row }">
            <ElTag size="small" type="primary">{{ row.action }}</ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="target" label="目标" />
        <ElTableColumn prop="ip" label="IP" width="140" />
        <ElTableColumn prop="created_at" label="时间" width="180" />
      </ElTable>
    </ElCard>
  </div>
</template>

<style scoped>
.dashboard h1 {
  margin-bottom: 20px;
}

.stat {
  text-align: center;
}

.stat h3 {
  margin: 0 0 12px;
  font-size: 14px;
  color: #909399;
}

.stat .value {
  font-size: 28px;
  font-weight: bold;
  margin: 10px 0;
}

.stat .sub {
  margin: 8px 0 0;
  font-size: 12px;
  color: #c0c4cc;
}

.overview-card {
  text-align: center;
}

.overview-stat h3 {
  margin: 0 0 12px;
  font-size: 14px;
  color: #909399;
}

.big-value {
  font-size: 32px;
  font-weight: bold;
  margin: 8px 0;
}

.big-value .green {
  color: #67c23a;
}

.value-text {
  font-size: 18px;
  font-weight: bold;
  margin: 8px 0;
}

.overview-stat .sub {
  font-size: 12px;
  color: #c0c4cc;
}
</style>
