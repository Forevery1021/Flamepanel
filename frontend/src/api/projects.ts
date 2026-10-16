/**
 * Project 项目 API（M-B 主线二 Stage A-5 · Doc/20 §5.6/5.7）。
 *
 * 类型来自 OpenAPI 生成单源（src/api/generated，见 PR #45 后端契约）。
 */
import api from './client'
import type {
  ProjectDto,
  CreateProjectRequest,
  UpdateProjectRequest,
  ProcessStatus,
  LogLine,
  WebhookDeployDto,
  ProcfileDto,
  HealthCheckDto,
  BuilderDecisionDto,
  ProjectDbLinkDto,
  LinkableDatabaseDto,
  LinkDatabaseRequest,
} from '@/api/generated'

/** 项目列表 */
export function listProjects() {
  return api.get<ProjectDto[]>('/projects')
}

/** 项目详情 */
export function getProject(id: number) {
  return api.get<ProjectDto>(`/projects/${id}`)
}

/** 创建项目 */
export function createProject(req: CreateProjectRequest) {
  return api.post<ProjectDto>('/projects', req)
}

/** 更新项目（运行中改启动命令后端 409） */
export function updateProject(id: number, req: UpdateProjectRequest) {
  return api.put<ProjectDto>(`/projects/${id}`, req)
}

/** 删除项目（运行中自动停止；部署目录保留——零锁定纪律） */
export function deleteProject(id: number) {
  return api.delete<{ ok: boolean }>(`/projects/${id}`)
}

/** 部署项目（git clone / local_path 校验；部署 ≠ 启动） */
export function deployProject(id: number) {
  return api.post<ProjectDto>(`/projects/${id}/deploy`, null, { timeout: 0 })
}

/** 启动 / 停止 / 重启 */
export function startProject(id: number) {
  return api.post<ProjectDto>(`/projects/${id}/start`)
}

export function stopProject(id: number) {
  return api.post<ProjectDto>(`/projects/${id}/stop`)
}

export function restartProject(id: number) {
  return api.post<ProjectDto>(`/projects/${id}/restart`)
}

/** 实时进程状态 */
export function getProjectStatus(id: number) {
  return api.get<ProcessStatus>(`/projects/${id}/status`)
}

/** 最近日志（后端固定 tail 200 行） */
export function getProjectLogs(id: number) {
  return api.get<LogLine[]>(`/projects/${id}/logs`)
}

/** 开启/重置 webhook 部署（令牌明文仅本次响应返回） */
export function enableProjectWebhook(id: number) {
  return api.post<WebhookDeployDto>(`/projects/${id}/webhook/enable`)
}

/** 关闭 webhook 部署（回落 manual 并清空令牌） */
export function disableProjectWebhook(id: number) {
  return api.post<ProjectDto>(`/projects/${id}/webhook/disable`)
}

/** 查看 Procfile 进程声明（Stage A-8 · Doc/20 §5.8 ③，ps:scale 伸缩面板数据源） */
export function getProjectProcfile(id: number) {
  return api.get<ProcfileDto>(`/projects/${id}/procfile`)
}

/** 手动健康探测（Stage A-9 · Doc/20 §5.8 ④，零停机切换探测同一端点） */
export function checkProjectHealth(id: number) {
  return api.post<HealthCheckDto>(`/projects/${id}/health-check`)
}

/** 查看构建器协商决策（Stage A-8 · Doc/20 §5.8 ②：Dockerfile → pack-cli → Runtime） */
export function getProjectBuilder(id: number) {
  return api.get<BuilderDecisionDto>(`/projects/${id}/builder`)
}

// ── 服务联链（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）──────────────

/** 项目联链列表（不含连接串，凭据不出后端服务边界） */
export function listProjectLinks(id: number) {
  return api.get<ProjectDbLinkDto[]>(`/projects/${id}/links`)
}

/** 可关联的数据库实例清单（仅 running 实例，联链弹窗数据源） */
export function listLinkableDatabases(id: number) {
  return api.get<LinkableDatabaseDto[]>(`/projects/${id}/links/available`)
}

/** 关联数据库（按需幂等建库；连接串在项目启动/部署时注入） */
export function linkProjectDatabase(id: number, req: LinkDatabaseRequest) {
  return api.post<ProjectDbLinkDto>(`/projects/${id}/links`, req)
}

/** 解除联链 */
export function unlinkProjectDatabase(id: number, linkId: number) {
  return api.delete<{ ok: boolean }>(`/projects/${id}/links/${linkId}`)
}
