import { defineStore } from 'pinia'
import api from '@/api/client'

export const useSystemStore = defineStore('system', {
  state: () => ({
    cpuUsage: 0,
    memoryTotal: 0,
    memoryUsed: 0,
    uptime: '',
    dockerContainers: 0,     // ← 加上这个属性
    loading: false,
  }),

  actions: {
    async fetchSystemInfo() {
      this.loading = true
      try {
        const res = await api.get('/system/info')
        const data = res.data
        
        this.cpuUsage = data.cpu_usage || 0
        this.memoryTotal = data.memory_total_mb || 0
        this.memoryUsed = data.memory_used_mb || 0
        this.dockerContainers = data.docker_containers || 0
        this.uptime = `${Math.floor(data.uptime_seconds / 86400)}天 ${Math.floor((data.uptime_seconds % 86400) / 3600)}小时`
      } catch (error) {
        console.error('获取系统信息失败', error)
      } finally {
        this.loading = false
      }
    }
  }
})