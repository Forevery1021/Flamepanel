import api from './client'
import type { InspectionReport } from '@/api/generated'

export type {
  Category,
  CheckStatus,
  Finding,
  InspectionReport,
  InspectionSummary,
  KernelInfo,
  Severity,
} from '@/api/generated'

/**
 * S-17 系统体检报告（需 `toolbox:read`）。服务端 10 min 缓存：`cache_age_secs > 0` 表示命中缓存；
 * 首次调用（安装后）会同步执行一次采集，通常 < 2 s。
 */
export function getInspectionReport() {
  return api.get<InspectionReport>('/inspection')
}

/** 立即重新体检（需 `toolbox:execute`），忽略服务端缓存 */
export function runInspection() {
  return api.post<InspectionReport>('/inspection/run')
}
