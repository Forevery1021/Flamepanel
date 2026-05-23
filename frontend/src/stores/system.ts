import { defineStore } from 'pinia'
import api from '@/api/client'
import type { SystemInfoResponse, ProcessInfo, DockerContainer } from '@/types'

export const useSystemStore = defineStore('system', {
  state: () => ({
    info: null as SystemInfoResponse | null,
    processes: [] as ProcessInfo[],
    loading: false,
  }),

  getters: {
    cpuUsage: (s) => s.info?.cpu_usage ?? 0,
    memoryTotal: (s) => s.info?.memory_total_mb ?? 0,
    memoryUsed: (s) => s.info?.memory_used_mb ?? 0,
    memoryUsagePercent: (s) => s.info?.memory_usage_percent ?? 0,
    diskTotal: (s) => s.info?.disk_total_gb ?? 0,
    diskUsed: (s) => s.info?.disk_used_gb ?? 0,
    diskUsagePercent: (s) => s.info?.disk_usage_percent ?? 0,
    uptime: (s) => s.info?.uptime_display ?? '',
    loadOne: (s) => s.info?.load_one ?? 0,
    loadFive: (s) => s.info?.load_five ?? 0,
    hostname: (s) => s.info?.hostname ?? '',
  },

  actions: {
    async fetchSystemInfo() {
      this.loading = true
      try {
        const res = await api.get<SystemInfoResponse>('/system/info')
        this.info = res.data
      } catch (e) {
        console.error('获取系统信息失败', e)
      } finally {
        this.loading = false
      }
    },

    async fetchProcesses() {
      try {
        const res = await api.get<ProcessInfo[]>('/system/processes')
        this.processes = res.data
      } catch (e) {
        console.error('获取进程列表失败', e)
      }
    },
  },
})
