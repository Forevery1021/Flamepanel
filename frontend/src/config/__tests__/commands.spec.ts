import { describe, it, expect, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  buildActionCommands,
  buildRecentCommands,
  nextMaterial,
  type CommandItem,
} from '@/config/commands'
import { useThemeStore } from '@/stores/theme'

/** S-15：最近访问映射 + 「切换材质」动作替代旧「玻璃开关」 */
const t = (key: string) => key

function nav(id: string): CommandItem {
  return { id, label: id, icon: 'oi-circle', keywords: [], hint: id, run: () => {} }
}

describe('config/commands', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('buildRecentCommands 按最近顺序映射导航命令，并丢弃无权限/已下线的旧记录', () => {
    const navigation = [nav('Dashboard'), nav('Firewall'), nav('Memos')]
    const recent = buildRecentCommands(navigation, ['Memos', 'Users', 'Dashboard'], t)
    expect(recent.map((c) => c.id)).toEqual(['recent:Memos', 'recent:Dashboard'])
    expect(recent.every((c) => c.hint === 'palette.recent')).toBe(true)
    // 副本不改原导航命令（「全部命令」分组仍显示原 hint / id）
    expect(navigation[2].id).toBe('Memos')
    expect(navigation[2].hint).toBe('Memos')
  })

  it('nextMaterial 按 solid → glass → acrylic → solid 循环', () => {
    expect(nextMaterial('solid')).toBe('glass')
    expect(nextMaterial('glass')).toBe('acrylic')
    expect(nextMaterial('acrylic')).toBe('solid')
  })

  it('动作命令含「切换材质」且不再有旧玻璃开关；执行后材质前进一档', () => {
    const theme = useThemeStore()
    theme.setMaterial('glass')
    const actions = buildActionCommands(t, theme)
    expect(actions.some((a) => a.id === 'toggle-glass')).toBe(false)
    const cycle = actions.find((a) => a.id === 'cycle-material')
    expect(cycle).toBeTruthy()
    expect(cycle!.hint).toContain('settingsTheme.materials.glass')
    expect(cycle!.hint).toContain('settingsTheme.materials.acrylic')
    expect(cycle!.keywords).toEqual(expect.arrayContaining(['acrylic', '材质']))
    cycle!.run()
    expect(theme.material).toBe('acrylic')
    buildActionCommands(t, theme)
      .find((a) => a.id === 'cycle-material')!
      .run()
    expect(theme.material).toBe('solid')
    expect(theme.glassEnabled).toBe(false)
  })
})
