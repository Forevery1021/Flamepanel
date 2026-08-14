/**
 * ⌘K 命令集中配置（Modernization M1）
 *
 * 单一命令来源：侧边栏菜单与命令面板共享 `menuRoutes`（见 router/index.ts），
 * 命令面板把「导航命令」从路由表展开，再加上注册的「动作命令」。
 *
 * 约定：
 * - 导航命令：由 `buildNavigationCommands()` 从 `menuRoutes` 展开（与侧边栏同源，不重复硬编码）。
 * - 动作命令：`actionCommands()` 集中定义，可带 `when`/`permission` 按需显示。
 * - 新命令注册：在下方数组追加一项即可，无需改 CommandPalette 视图。
 */
import type { Router } from 'vue-router'
import { menuRoutes } from '@/router'
import { useThemeStore } from '@/stores/theme'

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
  return menuRoutes
    .filter((r) => r.meta?.group)
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
      icon: themeStore.mode === 'dark' ? 'oi-sun' : 'oi-moon',
      keywords: [t('topbar.toggleDark'), 'dark', 'dark mode', '主题', '明暗', 'テーマ'],
      run: () => themeStore.setMode(themeStore.mode === 'dark' ? 'light' : 'dark'),
    },
    {
      id: 'toggle-glass',
      label: t('settingsTheme.glassEnabled'),
      icon: themeStore.glassEnabled ? 'oi-filter-slash' : 'oi-filter',
      keywords: [
        t('settingsTheme.glassEnabled'),
        'glass',
        'blur',
        '玻璃',
        'ガラス',
      ],
      run: () => themeStore.setGlassEnabled(!themeStore.glassEnabled),
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
        const next = densityCycle[(densityCycle.indexOf(themeStore.custom.density) + 1) % densityCycle.length]
        themeStore.updateCustom({ density: next })
      },
    },
  ]
}

function densityLabel(d: 'compact' | 'standard' | 'comfortable'): string {
  const map = { compact: 'Compact', standard: 'Standard', comfortable: 'Comfortable' } as const
  return map[d]
}
