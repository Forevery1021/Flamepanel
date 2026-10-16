import type { DatabaseInstance } from '@/types'

/** 面板可一键安装的数据库引擎。 */
export type DatabaseEngine = 'mysql' | 'redis' | 'postgres' | 'mongodb'

export interface DatabaseInstallForm {
  name: string
  version: string
  port: number
  /** MySQL 为 root_password，Redis / PostgreSQL 为 password；提交时按引擎映射。 */
  password: string
}

interface EngineMeta {
  label: string
  defaultName: string
  defaultPort: number
  /** 密码是否可选（Redis / PostgreSQL 可空；MySQL root 密码可由后端生成，但界面仍提示填写）。 */
  passwordOptional: boolean
  titleKey: string
  /** 固定可选版本（MongoDB 官方仓库系列）；缺省为自由输入。首项为默认值。 */
  versions?: string[]
  /** 非空密码的最小长度（MongoDB 管理员密码 ≥ 8 位，与后端校验一致）。 */
  minPasswordLength?: number
  /** 密码字段下方的说明（i18n key）。 */
  passwordHintKey?: string
}

export const DATABASE_ENGINES: Record<DatabaseEngine, EngineMeta> = {
  mysql: {
    label: 'MySQL',
    defaultName: 'MySQL 8',
    defaultPort: 3306,
    passwordOptional: false,
    titleKey: 'database.installMysql',
  },
  redis: {
    label: 'Redis',
    defaultName: 'Redis 7',
    defaultPort: 6379,
    passwordOptional: true,
    titleKey: 'database.installRedis',
  },
  postgres: {
    label: 'PostgreSQL',
    defaultName: 'PostgreSQL 16',
    defaultPort: 5432,
    passwordOptional: true,
    titleKey: 'database.installPostgres',
  },
  mongodb: {
    label: 'MongoDB',
    defaultName: 'MongoDB 8',
    defaultPort: 27017,
    passwordOptional: true,
    titleKey: 'database.installMongodb',
    versions: ['8.0', '9.0'],
    minPasswordLength: 8,
    passwordHintKey: 'database.mongoAuthHint',
  },
}

export function engineTypeOptions(): { label: string; value: DatabaseEngine }[] {
  return (Object.keys(DATABASE_ENGINES) as DatabaseEngine[]).map((value) => ({
    label: DATABASE_ENGINES[value].label,
    value,
  }))
}

export function defaultInstallForm(engine: DatabaseEngine): DatabaseInstallForm {
  const meta = DATABASE_ENGINES[engine]
  return {
    name: meta.defaultName,
    version: meta.versions?.[0] ?? '',
    port: meta.defaultPort,
    password: '',
  }
}

/** 密码是否满足引擎约束（可选引擎留空视为满足）。 */
export function passwordValid(engine: DatabaseEngine, password: string): boolean {
  const meta = DATABASE_ENGINES[engine]
  if (password === '') return meta.passwordOptional
  return password.length >= (meta.minPasswordLength ?? 1)
}

/** 把表单映射为各安装接口的请求体（MySQL 用 root_password 字段）。 */
export function installPayload(engine: DatabaseEngine, form: DatabaseInstallForm) {
  const base = { name: form.name.trim(), version: form.version.trim(), port: form.port }
  return engine === 'mysql'
    ? { ...base, root_password: form.password }
    : { ...base, password: form.password }
}

/** 当前页客户端过滤：类型精确匹配 + 名称 / 类型 / 端口关键字。 */
export function filterInstances(
  rows: DatabaseInstance[],
  keyword: string,
  type: string | null | undefined,
): DatabaseInstance[] {
  const kw = keyword.trim().toLowerCase()
  return rows.filter((d) => {
    if (type && d.db_type !== type) return false
    if (!kw) return true
    return (
      d.name.toLowerCase().includes(kw) ||
      d.db_type.toLowerCase().includes(kw) ||
      String(d.port).includes(kw)
    )
  })
}
