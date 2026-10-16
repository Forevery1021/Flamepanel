/**
 * F1.1 统一数据获取层 — 全局 queryKey 常量。
 *
 * 约定：queryKey 以模块名开头（小写），后跟参数。
 * 同一 key 在页面/组件间共享缓存（同屏多组件读同一资源只打一次有效请求）。
 */
export const queryKeys = {
  overview: {
    all: ['overview'] as const,
    /** S-06 顶栏聚合徽标（AppHeader 全局唯一订阅者，30 s 轮询） */
    badges: () => ['overview', 'badges'] as const,
  },
  metrics: {
    all: ['metrics'] as const,
    /** S-07 监控历史（按 range 分桶缓存） */
    history: (range: string) => ['metrics', 'history', range] as const,
  },
  inspection: {
    all: ['inspection'] as const,
    /** S-17 系统体检报告（服务端 10 min 缓存，前端只在页面 / 仪表盘各持一份） */
    report: () => ['inspection', 'report'] as const,
  },
  toolbox: {
    all: ['toolbox'] as const,
    /** S-12 进程列表（按查询参数分桶；轮询由页面开关控制） */
    processes: (params: Record<string, unknown>) => ['toolbox', 'processes', params] as const,
    /** 系统工具箱 · 主机详情（只读汇总，无服务端缓存，前端 30 s staleTime） */
    host: () => ['toolbox', 'host'] as const,
  },
  nodes: {
    all: ['nodes'] as const,
    list: (page = 1, pageSize = 20) => ['nodes', 'list', page, pageSize] as const,
  },
  containers: {
    all: ['containers'] as const,
    list: () => ['containers', 'list'] as const,
  },
  images: {
    all: ['images'] as const,
    list: () => ['images', 'list'] as const,
  },
  networks: {
    all: ['networks'] as const,
    list: () => ['networks', 'list'] as const,
  },
  volumes: {
    all: ['volumes'] as const,
    list: () => ['volumes', 'list'] as const,
  },
  composeProjects: {
    all: ['composeProjects'] as const,
    list: () => ['composeProjects', 'list'] as const,
  },
  packages: {
    all: ['packages'] as const,
    list: (category?: string) => ['packages', 'list', category ?? ''] as const,
  },
  installedApps: {
    all: ['installedApps'] as const,
    list: () => ['installedApps', 'list'] as const,
  },
  wasmBuiltins: {
    all: ['wasmBuiltins'] as const,
    list: () => ['wasmBuiltins', 'list'] as const,
  },
  settings: {
    all: ['settings'] as const,
    list: () => ['settings', 'list'] as const,
  },
  files: {
    all: ['files'] as const,
    list: (path: string) => ['files', 'list', path] as const,
  },
  operationLogs: {
    all: ['operationLogs'] as const,
    list: (page: number, pageSize: number, filter?: string) =>
      ['operationLogs', 'list', page, pageSize, filter ?? ''] as const,
  },
  users: {
    all: ['users'] as const,
    list: (page: number, pageSize: number) => ['users', 'list', page, pageSize] as const,
  },
  websites: {
    all: ['websites'] as const,
    list: (page: number, pageSize: number) => ['websites', 'list', page, pageSize] as const,
  },
  engines: {
    all: ['engines'] as const,
    list: () => ['engines', 'list'] as const,
  },
  databases: {
    all: ['databases'] as const,
    list: (page: number, pageSize: number) => ['databases', 'list', page, pageSize] as const,
  },
  plugins: {
    all: ['plugins'] as const,
    list: () => ['plugins', 'list'] as const,
    market: () => ['plugins', 'market'] as const,
  },
  scheduledTasks: {
    all: ['scheduledTasks'] as const,
    list: (page: number, pageSize: number) => ['scheduledTasks', 'list', page, pageSize] as const,
  },
  systemLogs: {
    all: ['systemLogs'] as const,
    list: (page: number, pageSize: number) => ['systemLogs', 'list', page, pageSize] as const,
  },
  backups: {
    all: ['backups'] as const,
    list: () => ['backups', 'list'] as const,
  },
  backupAccounts: {
    all: ['backupAccounts'] as const,
    list: () => ['backupAccounts', 'list'] as const,
    objects: (accountId: number) => ['backupAccounts', 'objects', accountId] as const,
  },
  assistant: {
    all: ['assistant'] as const,
    config: () => ['assistant', 'config'] as const,
  },
  mcp: {
    all: ['mcp'] as const,
    prompts: () => ['mcp', 'prompts'] as const,
    principals: () => ['mcp', 'principals'] as const,
    approvals: (status: string, page: number, pageSize: number) =>
      ['mcp', 'approvals', status, page, pageSize] as const,
    pendingApprovals: () => ['mcp', 'approvals', 'pending-count'] as const,
    tools: () => ['mcp', 'tools'] as const,
    calls: (page: number, pageSize: number, principal: string, tool: string) =>
      ['mcp', 'calls', page, pageSize, principal, tool] as const,
  },
  waf: {
    all: ['waf'] as const,
    rules: () => ['waf', 'rules'] as const,
    logs: (page: number, websiteId: number) => ['waf', 'logs', page, websiteId] as const,
    status: () => ['waf', 'status'] as const,
  },
  dnsProviders: {
    all: ['dnsProviders'] as const,
    list: () => ['dnsProviders', 'list'] as const,
  },
  backupJobs: {
    all: ['backupJobs'] as const,
    list: () => ['backupJobs', 'list'] as const,
  },
  tasks: {
    all: ['tasks'] as const,
    list: (state?: string) => ['tasks', 'list', state ?? ''] as const,
  },
  webServers: {
    all: ['webServers'] as const,
    list: (page: number, pageSize: number) => ['webServers', 'list', page, pageSize] as const,
  },
  runtimes: {
    all: ['runtimes'] as const,
    list: () => ['runtimes', 'list'] as const,
    kind: (kind: string) => ['runtimes', 'kind', kind] as const,
    available: (kind: string) => ['runtimes', 'available', kind] as const,
  },
  projects: {
    all: ['projects'] as const,
    list: () => ['projects', 'list'] as const,
    detail: (id: number) => ['projects', 'detail', id] as const,
    status: (id: number) => ['projects', 'status', id] as const,
    logs: (id: number) => ['projects', 'logs', id] as const,
    links: (id: number) => ['projects', 'links', id] as const,
  },
} as const
