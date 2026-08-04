import api from './client'
import type { HealthDetail } from '@/types'

export function fetchHealthDetail() {
  return api.get<HealthDetail>('/health')
}
