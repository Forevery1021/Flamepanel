import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：备份账号 / 计划备份任务 API 层（Doc/20 §6.5 前端接入）——断言请求路径与方法正确
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  const longApi = { post: vi.fn() }
  return { default: { get, post, put, delete: del }, longApi }
})

import api, { longApi } from '@/api/client'
import {
  listBackupAccounts,
  createBackupAccount,
  updateBackupAccount,
  deleteBackupAccount,
  testBackupAccount,
  listRemoteObjects,
  uploadRemoteObject,
  deleteRemoteObject,
} from '@/api/backupAccounts'
import {
  listBackupJobs,
  createBackupJob,
  updateBackupJob,
  deleteBackupJob,
  toggleBackupJob,
  runBackupJob,
  restoreFromCloud,
} from '@/api/backupJobs'

describe('backupAccounts API（备份账号）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listBackupAccounts 走 GET /backup-accounts', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listBackupAccounts()
    expect(api.get).toHaveBeenCalledWith('/backup-accounts')
  })

  it('createBackupAccount 走 POST /backup-accounts', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'oss', account_type: 's3', config_json: '{}', enabled: true }
    await createBackupAccount(req)
    expect(api.post).toHaveBeenCalledWith('/backup-accounts', req)
  })

  it('updateBackupAccount 走 PUT /backup-accounts/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateBackupAccount(3, { enabled: false })
    expect(api.put).toHaveBeenCalledWith('/backup-accounts/3', { enabled: false })
  })

  it('deleteBackupAccount 走 DELETE /backup-accounts/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await deleteBackupAccount(5)
    expect(api.delete).toHaveBeenCalledWith('/backup-accounts/5')
  })

  it('testBackupAccount 走 POST /backup-accounts/:id/test', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { last_test_status: 'success' } })
    await testBackupAccount(7)
    expect(api.post).toHaveBeenCalledWith('/backup-accounts/7/test')
  })

  it('listRemoteObjects 走 GET /backup-accounts/:id/objects', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listRemoteObjects(2)
    expect(api.get).toHaveBeenCalledWith('/backup-accounts/2/objects')
  })

  it('uploadRemoteObject 走 POST /backup-accounts/:id/objects/upload', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { key: 'panel/a.db', size: 1 } })
    await uploadRemoteObject(2, 'panel-a.db', 'panel/panel-a.db')
    expect(api.post).toHaveBeenCalledWith('/backup-accounts/2/objects/upload', {
      local_filename: 'panel-a.db',
      remote_key: 'panel/panel-a.db',
    })
  })

  it('deleteRemoteObject 走 DELETE /backup-accounts/:id/objects?key=', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await deleteRemoteObject(2, 'panel/a.db')
    expect(api.delete).toHaveBeenCalledWith('/backup-accounts/2/objects', { params: { key: 'panel/a.db' } })
  })
})

describe('backupJobs API（计划备份任务）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listBackupJobs 走 GET /backup-jobs', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] })
    await listBackupJobs()
    expect(api.get).toHaveBeenCalledWith('/backup-jobs')
  })

  it('createBackupJob 走 POST /backup-jobs', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    const req = { name: 'nightly', scope: 'panel', schedule: '0 3 * * *', target_account_id: 1, retention: 7, enabled: true }
    await createBackupJob(req)
    expect(api.post).toHaveBeenCalledWith('/backup-jobs', req)
  })

  it('updateBackupJob 走 PUT /backup-jobs/:id', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await updateBackupJob(4, { schedule: '0 4 * * *' })
    expect(api.put).toHaveBeenCalledWith('/backup-jobs/4', { schedule: '0 4 * * *' })
  })

  it('toggleBackupJob 走 PUT /backup-jobs/:id/enabled', async () => {
    ;(api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await toggleBackupJob(4, true)
    expect(api.put).toHaveBeenCalledWith('/backup-jobs/4/enabled', { enabled: true })
  })

  it('runBackupJob 走 longApi POST（长操作：立即执行备份）', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await runBackupJob(9)
    expect(longApi.post).toHaveBeenCalledWith('/backup-jobs/9/run')
  })

  it('restoreFromCloud 走 longApi POST /backup-jobs/restore-cloud', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await restoreFromCloud(2, 'panel/flamepanel-20260907.db')
    expect(longApi.post).toHaveBeenCalledWith('/backup-jobs/restore-cloud', {
      account_id: 2,
      remote_key: 'panel/flamepanel-20260907.db',
    })
  })

  it('restoreFromCloud 同样支持应用数据（app/<key>/*.tar.gz）', async () => {
    ;(longApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await restoreFromCloud(2, 'app/wordpress/wordpress-20260907-030000.tar.gz')
    expect(longApi.post).toHaveBeenCalledWith('/backup-jobs/restore-cloud', {
      account_id: 2,
      remote_key: 'app/wordpress/wordpress-20260907-030000.tar.gz',
    })
  })

  it('deleteBackupJob 走 DELETE /backup-jobs/:id', async () => {
    ;(api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await deleteBackupJob(6)
    expect(api.delete).toHaveBeenCalledWith('/backup-jobs/6')
  })
})
