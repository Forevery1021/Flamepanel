/**
 * 备份账号 API（Doc/20 §6.5 Stage 1/2 · /api/backup-accounts）。
 * 类型来自 OpenAPI 生成（src/api/generated），禁止手写重复定义。
 */
import api from './client'
import type {
  BackupAccountDto,
  CreateAccountRequest,
  UpdateAccountRequest,
  RemoteObjectDto,
} from '@/api/generated'

export type { BackupAccountDto, CreateAccountRequest, UpdateAccountRequest, RemoteObjectDto }

export function listBackupAccounts() {
  return api.get<BackupAccountDto[]>('/backup-accounts')
}

export function createBackupAccount(input: CreateAccountRequest) {
  return api.post<BackupAccountDto>('/backup-accounts', input)
}

export function updateBackupAccount(id: number, input: UpdateAccountRequest) {
  return api.put<BackupAccountDto>(`/backup-accounts/${id}`, input)
}

export function deleteBackupAccount(id: number) {
  return api.delete(`/backup-accounts/${id}`)
}

/** 连通性测试（真实探测：连接/认证/写探针），失败返回 200 + last_test_status=failed */
export function testBackupAccount(id: number) {
  return api.post<BackupAccountDto>(`/backup-accounts/${id}/test`)
}

export function listRemoteObjects(accountId: number) {
  return api.get<RemoteObjectDto[]>(`/backup-accounts/${accountId}/objects`)
}

export function uploadRemoteObject(accountId: number, localFilename: string, remoteKey?: string) {
  return api.post<RemoteObjectDto>(`/backup-accounts/${accountId}/objects/upload`, {
    local_filename: localFilename,
    remote_key: remoteKey ?? null,
  })
}

export function deleteRemoteObject(accountId: number, key: string) {
  return api.delete(`/backup-accounts/${accountId}/objects`, { params: { key } })
}
