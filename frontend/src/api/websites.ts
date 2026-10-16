import api, { longApi } from './client'
import type { Website } from '@/api/generated'
import type { Page } from '@/api/generated'

export function listWebsites(page = 1, pageSize = 20) {
  return api.get<Page<Website>>('/websites', { params: { page, page_size: pageSize } })
}

export function createWebsite(website: Website) {
  return api.post<number>('/websites', { website })
}

export function updateWebsite(id: number, website: Website) {
  return api.put<Website>(`/websites/${id}`, { website })
}

export function deleteWebsite(id: number) {
  return api.delete(`/websites/${id}`)
}

/** 站点部署结果（后端 `SiteDeployment`） */
export interface SiteDeployment {
  engine: string
  /** 站点配置文件（逻辑路径） */
  path: string
  enabled: boolean
  /** 引擎未运行时为 false：配置已通过校验，下次启动生效 */
  reloaded: boolean
}

/** 手动部署站点配置：写入 → 配置校验 → reload，失败整体回滚（Doc/20 §5.5） */
export function deployWebsite(id: number) {
  return longApi.post<SiteDeployment>(`/websites/${id}/deploy`)
}

export function switchWebsiteEngine(id: number, engine: string) {
  // F-33: 引擎切换为长操作
  return longApi.post<Website>(`/websites/${id}/switch-engine`, { engine })
}

/** 站点类型（与后端 Website::is_valid_site_type 白名单对齐 · Doc/20 §5.5） */
export const SITE_TYPES = ['runtime_php', 'reverse_proxy', 'static', 'container_app'] as const
export type SiteType = (typeof SITE_TYPES)[number]

/** 绑定运行时/项目（site_type=static 且不传 id 即解绑） */
export function bindWebsiteRuntime(
  id: number,
  req: { site_type: SiteType; runtime_id?: number | null; project_id?: number | null },
) {
  return api.post<Website>(`/websites/${id}/runtime`, req)
}

/** 一键套用伪静态规则模板（落库 + 写 vhost + 校验 + reload；返回生成的规则文本） */
export function applyWebsiteRewrite(id: number, preset: string) {
  return api.post<string>(`/websites/${id}/rewrite`, { preset })
}

/** 查询站点伪静态套用状态（当前模板 + 按引擎语法规则文本 + 可选模板清单） */
export function getWebsiteRewrite(id: number) {
  return api.get<SiteRewriteStatus>(`/websites/${id}/rewrite`)
}

/** 清除伪静态套用（回落引擎默认路由；同样落库 + 写 vhost） */
export function clearWebsiteRewrite(id: number) {
  return api.delete<SiteRewriteStatus>(`/websites/${id}/rewrite`)
}

/** 伪静态套用状态（Doc/20 §5.5 收口） */
export interface SiteRewriteStatus {
  preset: string | null
  rules: string
  supported: string[]
}

/** 伪静态模板清单（与后端 RewritePreset::ALL 对齐；下拉回退用，权威值取接口 supported） */
export const REWRITE_PRESETS = [
  'wordpress',
  'discuz',
  'typecho',
  'laravel',
  'thinkphp',
  'springboot',
] as const
