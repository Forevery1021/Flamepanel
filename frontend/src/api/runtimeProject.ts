/**
 * 运行时/项目联合查询（M-B 主线二 Stage A-6 · Doc/20 §5.5）。
 *
 * 复用既有 runtimes/projects API（`api/runtimes.ts`、`api/projects.ts`），
 * 为 WebsitesView 绑定弹窗提供运行时与项目下拉数据源。
 */
import { listRuntimes } from './runtimes'
import { listProjects } from './projects'
import type { RuntimeDto, ProjectDto } from '@/api/generated'

export { listRuntimes, listProjects }
export type { RuntimeDto, ProjectDto }
