/**
 * ⌘K 命令集中配置（Modernization M1）
 *
 * 单一命令来源：侧边栏菜单与命令面板共享 `menuRoutes`（见 router/index.ts），
 * 命令面板把「导航命令」从路由表展开，再加上注册的「动作命令」。
 *
 * 约定：
 * - 导航命令：由 `buildNavigationCommands()` 从 `menuRoutes` 展开（与侧边栏同源，不重复硬编码）。
 * - 动作命令：`actionCommands()` 集中定义，可带 `when`/`permission` 按需显示。
 * - 最近访问（S-15）：`buildRecentCommands()` 把最近进入的菜单路由名映射回导航命令
 *   （与导航命令求交集 → 无权限 / 已下线的旧记录自然隐藏）。
 * - 新命令注册：在下方数组追加一项即可，无需改 CommandPalette 视图。
 */
import type { Router } from 'vue-router'
import { menuRoutes } from '@/router'
import { isRouteVisibleInMenu } from '@/directives/permission'
import { useAuthStore } from '@/stores/auth'
import { THEME_MATERIALS, type ThemeMaterial, useThemeStore } from '@/stores/theme'

type ThemeStore = ReturnType<typeof useThemeStore>

export interface CommandItem {
  id: string
  label: string
  icon: string
  hint?: string
  keywords?: string[]
  run: () => void
}

/**
 * 从路由表展开导航命令（含 ⌘K 搜索权重、关键词、图标）。
 * 与侧边栏同源：改菜单只需改路由 meta，命令面板自动跟随。
 */
export function buildNavigationCommands(t: (key: string) => string, router: Router): CommandItem[] {
  // T0-1：⌘K 与侧栏/守卫同源过滤——否则无权限页面仍会被命令面板列出并「可跳转」，
  // 点进去又被守卫弹回（死路）。判据见 `directives/permission.ts::isRouteVisibleInMenu`。
  const role = useAuthStore().role
  return menuRoutes
    .filter((r) => r.meta?.group)
    .filter((r) => isRouteVisibleInMenu(role, r))
    .sort((a, b) => (b.meta?.weight ?? 0) - (a.meta?.weight ?? 0))
    .map((r) => ({
      id: String(r.name),
      label: t(r.meta?.title ?? ''),
      icon: r.meta?.icon ?? 'oi-circle',
      keywords: r.meta?.keywords ?? [],
      hint: r.meta?.keywords?.[0] ?? '',
      run: () => router.push(r.path),
    }))
}

/**
 * 最近访问（S-15）：按 `recentNames` 顺序取出对应导航命令的副本。
 * id 加 `recent:` 前缀，避免与「全部命令」分组中的同一项 key 冲突。
 */
export function buildRecentCommands(
  navigation: readonly CommandItem[],
  recentNames: readonly string[],
  t: (key: string) => string,
): CommandItem[] {
  const byId = new Map(navigation.map((c) => [c.id, c]))
  return recentNames
    .map((name) => byId.get(name))
    .filter((c): c is CommandItem => !!c)
    .map((c) => ({ ...c, id: `recent:${c.id}`, hint: t('palette.recent') }))
}

/** 材质循环：solid → glass → acrylic → solid（替代 v0.9 之前的「玻璃开关」布尔语义） */
export function nextMaterial(current: ThemeMaterial): ThemeMaterial {
  const i = THEME_MATERIALS.indexOf(current)
  return THEME_MATERIALS[(i + 1) % THEME_MATERIALS.length]
}

/** 动作命令（非导航、执行式）。集中在此注册。 */
export function buildActionCommands(
  t: (key: string) => string,
  themeStore: ThemeStore,
): CommandItem[] {
  const densityCycle: Array<'compact' | 'standard' | 'comfortable'> = [
    'compact',
    'standard',
    'comfortable',
  ]
  return [
    {
      id: 'toggle-dark',
      label: t('topbar.toggleDark'),
      icon: themeStore.isDark ? 'oi-sun' : 'oi-moon',
      keywords: [t('topbar.toggleDark'), 'dark', 'dark mode', '主题', '明暗', 'テーマ'],
      run: () => themeStore.toggleDark(),
    },
    {
      id: 'cycle-material',
      label: t('palette.cycleMaterial'),
      icon: 'oi-layers',
      hint: `${t('settingsTheme.material')}: ${t(`settingsTheme.materials.${themeStore.material}`)} → ${t(`settingsTheme.materials.${nextMaterial(themeStore.material)}`)}`,
      keywords: [
        t('palette.cycleMaterial'),
        t('settingsTheme.material'),
        ...THEME_MATERIALS.map((m) => t(`settingsTheme.materials.${m}`)),
        'material',
        'glass',
        'acrylic',
        'solid',
        'blur',
        '材质',
        '玻璃',
        '素材',
        'ガラス',
      ],
      run: () => themeStore.setMaterial(nextMaterial(themeStore.material)),
    },
    {
      id: 'cycle-density',
      label: t('settingsTheme.density'),
      icon: 'oi-expand',
      hint: `${t('settingsTheme.density')}: ${t(`settingsTheme.density${densityLabel(themeStore.custom.density)}`)}`,
      keywords: [
        t('settingsTheme.density'),
        t('settingsTheme.densityCompact'),
        t('settingsTheme.densityStandard'),
        t('settingsTheme.densityComfortable'),
        'density',
        '密度',
        '密度切換',
      ],
      run: () => {
        const next =
          densityCycle[(densityCycle.indexOf(themeStore.custom.density) + 1) % densityCycle.length]
        themeStore.updateCustom({ density: next })
      },
    },
  ]
}

function densityLabel(d: 'compact' | 'standard' | 'comfortable'): string {
  const map = { compact: 'Compact', standard: 'Standard', comfortable: 'Comfortable' } as const
  return map[d]
}
