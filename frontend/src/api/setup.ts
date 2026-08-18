import api from './client'

/** 初始化状态（后端 `GET /api/setup/status`） */
export interface SetupStatus {
  status: 'in_progress' | 'completed' | 'unattended'
  theme: string
  language: string
  docker: boolean | null
  nginx: boolean | null
}

export interface SetupDatabaseInput {
  db_type: 'sqlite' | 'mysql' | 'mariadb'
  host?: string
  port?: number
  name?: string
  user?: string
  password?: string
  mysql_root_password?: string
}

export interface SetupAdminInput {
  username: string
  password: string
}

export interface SetupInitializePayload {
  step: 'database' | 'admin'
  database?: SetupDatabaseInput
  admin?: SetupAdminInput
  theme?: string
  language?: string
}

export interface SetupInitializeResponse {
  status: string
  message: string
  token?: string
  refresh_token?: string
  username?: string
  role?: string
}

/** 查询初始化状态（公开端点，无需登录） */
export function getSetupStatus() {
  return api.get<SetupStatus>('/setup/status')
}

/** 执行初始化步骤（公开端点；限流 5/min/IP，前端仅在用户显式提交时调用） */
export function initializeSetup(payload: SetupInitializePayload) {
  return api.post<SetupInitializeResponse>('/setup/initialize', payload)
}