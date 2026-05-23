import { defineStore } from 'pinia'
import api from '@/api/client'
import type { DashboardInfo } from '@/types'

export const useDashboardStore = defineStore('dashboard', {
  state: () => ({
    data: null as DashboardInfo | null,
    loading: false,
  }),

  getters: {
    serverInfo: (s) => s.data?.server_info,
    dockerRunning: (s) => s.data?.docker_containers_running ?? 0,
    dockerTotal: (s) => s.data?.docker_containers_total ?? 0,
    websitesRunning: (s) => s.data?.websites_running ?? 0,
    websitesTotal: (s) => s.data?.websites_total ?? 0,
    wafRulesCount: (s) => s.data?.waf_rules_count ?? 0,
    wafRulesEnabled: (s) => s.data?.waf_rules_enabled ?? 0,
    recentLogs: (s) => s.data?.recent_logs ?? [],
  },

  actions: {
    async fetchDashboard() {
      this.loading = true
      try {
        const res = await api.get<DashboardInfo>('/dashboard/overview')
        this.data = res.data
      } catch (e) {
        console.error('获取仪表盘数据失败', e)
      } finally {
        this.loading = false
      }
    },
  },
})
