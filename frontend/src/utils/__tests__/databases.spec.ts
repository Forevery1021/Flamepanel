import { describe, it, expect } from 'vitest'
import {
  DATABASE_ENGINES,
  defaultInstallForm,
  engineTypeOptions,
  filterInstances,
  installPayload,
  passwordValid,
} from '@/utils/databases'
import type { DatabaseInstance } from '@/types'

const db = (over: Partial<DatabaseInstance>) =>
  ({
    id: 1,
    name: 'main',
    db_type: 'mysql',
    version: '8',
    port: 3306,
    status: 'running',
    data_dir: '/d',
    ...over,
  }) as DatabaseInstance

describe('utils/databases', () => {
  it('引擎元数据：默认名称 / 端口 / 类型选项', () => {
    expect(defaultInstallForm('redis')).toEqual({
      name: 'Redis 7',
      version: '',
      port: 6379,
      password: '',
    })
    expect(defaultInstallForm('postgres').port).toBe(5432)
    expect(engineTypeOptions().map((o) => o.value)).toEqual([
      'mysql',
      'redis',
      'postgres',
      'mongodb',
    ])
    expect(DATABASE_ENGINES.mysql.passwordOptional).toBe(false)
    expect(defaultInstallForm('mongodb')).toEqual({
      name: 'MongoDB 8',
      version: '8.0',
      port: 27017,
      password: '',
    })
  })

  it('passwordValid：可选引擎留空通过，MongoDB 非空需 ≥ 8 位，MySQL 不可空', () => {
    expect(passwordValid('mongodb', '')).toBe(true)
    expect(passwordValid('mongodb', 'short')).toBe(false)
    expect(passwordValid('mongodb', '12345678')).toBe(true)
    expect(passwordValid('redis', 'x')).toBe(true)
    expect(passwordValid('mysql', '')).toBe(false)
  })

  it('installPayload：MySQL 映射 root_password，其它映射 password，名称版本去空白', () => {
    const form = { name: ' a ', version: ' 8.4 ', port: 3307, password: 'pw' }
    expect(installPayload('mysql', form)).toEqual({
      name: 'a',
      version: '8.4',
      port: 3307,
      root_password: 'pw',
    })
    expect(installPayload('redis', form)).toEqual({
      name: 'a',
      version: '8.4',
      port: 3307,
      password: 'pw',
    })
  })

  it('filterInstances：类型精确 + 名称 / 类型 / 端口关键字', () => {
    const rows = [db({ id: 1 }), db({ id: 2, name: 'Cache', db_type: 'redis', port: 6379 })]
    expect(filterInstances(rows, '', null)).toHaveLength(2)
    expect(filterInstances(rows, '', 'redis').map((r) => r.id)).toEqual([2])
    expect(filterInstances(rows, ' CACHE ', '').map((r) => r.id)).toEqual([2])
    expect(filterInstances(rows, '3306', undefined).map((r) => r.id)).toEqual([1])
    expect(filterInstances(rows, 'mysql', 'redis')).toEqual([])
  })
})
