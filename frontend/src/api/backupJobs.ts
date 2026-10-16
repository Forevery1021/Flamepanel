/**
 * 计划备份任务 API（Doc/20 §6.5 Stage 3 · /api/backup-jobs）。
 * 类型来自 OpenAPI 生成（src/api/generated），禁止手写重复定义。
 */
import api, { longApi } from './client'
import type {
  BackupJobDto,
  CreateJobRequest,
  UpdateJobRequest,
  RestoreFromCloudResponse,
} from '@/api/generated'

export type { BackupJobDto, CreateJobRequest, UpdateJobRequest, RestoreFromCloudResponse }

export function listBackupJobs() {
  return api.get<BackupJobDto[]>('/backup-jobs')
}

export function createBackupJob(input: CreateJobRequest) {
  return api.post<BackupJobDto>('/backup-jobs', input)
}

export function updateBackupJob(id: number, input: UpdateJobRequest) {
  return api.put<BackupJobDto>(`/backup-jobs/${id}`, input)
}

export function deleteBackupJob(id: number) {
  return api.delete(`/backup-jobs/${id}`)
}

export function toggleBackupJob(id: number, enabled: boolean) {
  return api.put<BackupJobDto>(`/backup-jobs/${id}/enabled`, { enabled })
}

/** 立即执行一次（备份可能耗时较长，走长超时通道） */
export function runBackupJob(id: number) {
  return longApi.post<BackupJobDto>(`/backup-jobs/${id}/run`)
}

/** 从云端账号直接拉取并恢复面板库（需重启生效） */
export function restoreFromCloud(accountId: number, remoteKey: string) {
  return longApi.post<RestoreFromCloudResponse>('/backup-jobs/restore-cloud', {
    account_id: accountId,
    remote_key: remoteKey,
  })
}
