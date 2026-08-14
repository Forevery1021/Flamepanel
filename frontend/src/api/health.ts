import api from './client'
import type { HealthDetail } from '@/api/generated'

export function fetchHealthDetail() {
  return api.get<HealthDetail>('/health')
}
