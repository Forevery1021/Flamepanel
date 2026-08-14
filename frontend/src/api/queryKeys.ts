/**
 * F1.1 统一数据获取层 — 全局 queryKey 常量。
 *
 * 约定：queryKey 以模块名开头（小写），后跟参数。
 * 同一 key 在页面/组件间共享缓存（同屏多组件读同一资源只打一次有效请求）。
 */
export const queryKeys = {
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
  tasks: {
    all: ['tasks'] as const,
    list: (state?: string) => ['tasks', 'list', state ?? ''] as const,
  },
  webServers: {
    all: ['webServers'] as const,
    list: (page: number, pageSize: number) => ['webServers', 'list', page, pageSize] as const,
  },
} as const
