import api from './client'
import type { FirewallRule, FirewallStatus } from '@/types'

export function listFirewallRules() {
  return api.get<FirewallRule[]>('/firewall/rules')
}

export function getFirewallRule(id: number) {
  return api.get<FirewallRule>(`/firewall/rules/${id}`)
}

export function createFirewallRule(data: Partial<FirewallRule>) {
  return api.post<FirewallRule>('/firewall/rules', data)
}

export function updateFirewallRule(id: number, data: Partial<FirewallRule>) {
  return api.put<FirewallRule>(`/firewall/rules/${id}`, data)
}

export function deleteFirewallRule(id: number) {
  return api.delete(`/firewall/rules/${id}`)
}

export function toggleFirewallRule(id: number, enabled: boolean) {
  return api.post<FirewallRule>(`/firewall/rules/${id}/toggle`, { enabled })
}

export function applyFirewallRules() {
  return api.post('/firewall/apply')
}

export function getFirewallStatus() {
  return api.get<FirewallStatus>('/firewall/status')
}

export function enableFirewall() {
  return api.post('/firewall/enable')
}

export function disableFirewall() {
  return api.post('/firewall/disable')
}

export function reorderFirewallRules(ids: number[]) {
  return api.post('/firewall/reorder', { ids })
}
