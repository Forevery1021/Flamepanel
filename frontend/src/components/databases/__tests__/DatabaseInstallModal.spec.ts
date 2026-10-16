import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const { api, toast } = vi.hoisted(() => ({
  api: {
    installMysql: vi.fn(),
    installRedis: vi.fn(),
    installPostgres: vi.fn(),
    installMongodb: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))
vi.mock('@/api/databases', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import DatabaseInstallModal from '@/components/databases/DatabaseInstallModal.vue'

const messages = {
  database: {
    installMysql: '安装 MySQL',
    installRedis: '安装 Redis',
    installPostgres: '安装 PostgreSQL',
    installMongodb: '安装 MongoDB',
    mongoAuthHint: '填写后开启鉴权',
    passwordTooShort: '密码至少 {n} 位',
    name: '名称',
    version: '版本',
    versionPlaceholder: '版本号（可选）',
    port: '端口',
    rootPassword: 'Root 密码',
    password: '密码',
    optional: '可选',
  },
  common: {
    placeholder: '请输入',
    cancel: '取消',
    install: '安装',
    success: '成功',
    failed: '失败',
  },
}

async function open(engine: string) {
  document.body.innerHTML = ''
  const w = mount(DatabaseInstallModal, {
    props: { engine: null },
    attachTo: document.body,
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  await w.setProps({ engine: engine as never })
  await flushPromises()
  await nextTick()
  return w
}
const inputs = () =>
  Array.from(document.body.querySelectorAll('.modal-form input')) as HTMLInputElement[]
const installBtn = () =>
  Array.from(document.body.querySelectorAll('button')).find(
    (b) => b.textContent?.trim() === '安装',
  ) as HTMLButtonElement

beforeEach(() => {
  vi.clearAllMocks()
  for (const fn of Object.values(api)) fn.mockResolvedValue({ data: {} })
})

describe('DatabaseInstallModal', () => {
  it('MySQL：标题、默认值与 root_password 请求体，成功 emit done 并关闭', async () => {
    const w = await open('mysql')
    expect(document.body.textContent).toContain('安装 MySQL')
    expect(document.body.textContent).toContain('Root 密码')
    expect(inputs()[0]!.value).toBe('MySQL 8')
    const pw = inputs().find((i) => i.type === 'password')!
    pw.value = 'secret'
    pw.dispatchEvent(new Event('input'))
    await nextTick()
    installBtn().click()
    await flushPromises()
    expect(api.installMysql).toHaveBeenCalledWith({
      name: 'MySQL 8',
      version: '',
      port: 3306,
      root_password: 'secret',
    })
    expect(w.emitted('done')).toHaveLength(1)
    expect(w.emitted('update:engine')?.at(-1)).toEqual([null])
  })

  it('Redis：可选密码提示；失败保持打开', async () => {
    api.installRedis.mockRejectedValue(new Error('port busy'))
    const w = await open('redis')
    expect(document.body.textContent).toContain('安装 Redis')
    expect(inputs()[0]!.value).toBe('Redis 7')
    expect(inputs().find((i) => i.type === 'password')!.placeholder).toBe('可选')
    installBtn().click()
    await flushPromises()
    expect(api.installRedis).toHaveBeenCalledWith({
      name: 'Redis 7',
      version: '',
      port: 6379,
      password: '',
    })
    expect(toast.error).toHaveBeenCalled()
    expect(w.emitted('done')).toBeUndefined()
  })

  it('PostgreSQL：名称清空禁用安装', async () => {
    await open('postgres')
    const name = inputs()[0]!
    name.value = '  '
    name.dispatchEvent(new Event('input'))
    await nextTick()
    expect(installBtn().disabled).toBe(true)
    installBtn().click()
    await flushPromises()
    expect(api.installPostgres).not.toHaveBeenCalled()
  })

  it('MongoDB：默认 8.0 系列、鉴权说明；短密码报错并禁用，合格密码提交', async () => {
    const w = await open('mongodb')
    expect(document.body.textContent).toContain('安装 MongoDB')
    expect(document.body.textContent).toContain('填写后开启鉴权')
    expect(inputs()[0]!.value).toBe('MongoDB 8')
    const pw = inputs().find((i) => i.type === 'password')!
    pw.value = 'short'
    pw.dispatchEvent(new Event('input'))
    await nextTick()
    expect(document.body.textContent).toContain('密码至少 8 位')
    expect(installBtn().disabled).toBe(true)
    pw.value = 'long-enough-pw'
    pw.dispatchEvent(new Event('input'))
    await nextTick()
    expect(document.body.textContent).not.toContain('密码至少 8 位')
    installBtn().click()
    await flushPromises()
    expect(api.installMongodb).toHaveBeenCalledWith({
      name: 'MongoDB 8',
      version: '8.0',
      port: 27017,
      password: 'long-enough-pw',
    })
    expect(w.emitted('done')).toHaveLength(1)
  })
})
