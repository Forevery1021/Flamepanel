import { describe, it, expect, vi } from 'vitest'
import api from '@/api/client'

vi.mock('@/api/auth', () => ({
  refreshToken: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

const calls: Array<{ method?: string; url?: string; data?: unknown }> = []

api.defaults.adapter = async (config) => {
  calls.push({ method: config.method, url: config.url, data: config.data })
  return {
    data:
      config.method === 'post' && config.url === '/api/runtimes/install'
        ? { id: 1, kind: 'php', version: '8.2', status: 'installing', install_path: '', is_default: false, created_at: '', updated_at: '' }
        : [],
    status: 200,
    statusText: 'OK',
    headers: {},
    config,
  } as never
}

import { installRuntime, availableRuntimes, RUNTIME_KINDS } from '@/api/runtimes'
import { createProject } from '@/api/projects'

describe('runtimes/projects API（Stage A-5 前端接入）', () => {
  it('installRuntime 走 POST /runtimes/install 且携带 kind+version', async () => {
    await installRuntime({ kind: 'php', version: '8.2' })
    const c = calls.find((x) => (x.url ?? '').includes('/runtimes/install'))
    expect(c).toBeTruthy()
    expect(c?.method).toBe('post')
    expect(JSON.parse(String(c?.data))).toEqual({ kind: 'php', version: '8.2' })
  })

  it('availableRuntimes 以 query 参数传递 kind', async () => {
    await availableRuntimes('node')
    const c = calls.find((x) => (x.url ?? '').includes('/runtimes/available'))
    expect(c).toBeTruthy()
  })

  it('RUNTIME_KINDS 覆盖 Stage B-2 新增的 tomcat（Java WAR 容器运行时）', () => {
    expect(RUNTIME_KINDS).toContain('tomcat')
    // Stage B-2 语言全量：php/java/node/python/go/tomcat
    expect(RUNTIME_KINDS.length).toBe(6)
  })

  it('createProject 走 POST /projects', async () => {
    await createProject({ name: 'demo', kind: 'static', source: 'local_path' })
    const c = calls.find((x) => (x.url ?? '').endsWith('/projects') && x.method === 'post')
    expect(c).toBeTruthy()
    expect(JSON.parse(String(c?.data)).name).toBe('demo')
  })
})
