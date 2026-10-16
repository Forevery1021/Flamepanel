import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'

/** `auto` 跟随系统 `prefers-color-scheme`，实时响应系统切换 */
export type ThemeMode = 'light' | 'dark' | 'auto'
export const THEME_MODES: readonly ThemeMode[] = ['light', 'dark', 'auto'] as const

/**
 * 强调色预设（W4-a 起**只决定颜色**）。
 *
 * 旧版预设同时捆绑圆角 / 密度 / 模糊，且三个内置预设色相几乎相同（35° / 35° / 45°），
 * 滑块又只在 `custom` 下生效——实机表现为「点预设没变化、拖滑块也没变化」。现在：
 * - 预设 = 一组 OKLCH 种子色（色相明显不同）；
 * - 圆角 / 密度 / 模糊 / 材质是独立的偏好，任何预设下都即时生效；
 * - 在非自定义预设下拖动品牌色滑块，会以当前预设为起点自动切到 `custom`。
 */
export type ThemePreset =
  'flame' | 'aurora' | 'infinity' | 'ocean' | 'violet' | 'rose' | 'emerald' | 'graphite' | 'custom'
export type BuiltinPreset = Exclude<ThemePreset, 'custom'>

/**
 * 表面材质（OKLCH 材质体系，见 theme/tokens.css「Material」段）：
 * - `solid`：不透明表面、无模糊（低端设备 / 减弱透明度偏好）；
 * - `glass`：半透明 + 中等模糊 + 高饱和（默认）；
 * - `acrylic`：更厚的模糊 + 亮度叠加 + 细噪点（Fluent Acrylic 风格）。
 */
export type ThemeMaterial = 'solid' | 'glass' | 'acrylic'
export const THEME_MATERIALS: readonly ThemeMaterial[] = ['solid', 'glass', 'acrylic'] as const
export type RadiusScale = 'sharp' | 'standard' | 'rounded'
export type DensityScale = 'compact' | 'standard' | 'comfortable'

export interface BrandSeed {
  hue: number
  saturation: number
  lightness: number
}

export interface CustomTheme extends BrandSeed {
  glassBlur: number
  radius: RadiusScale
  density: DensityScale
  /** 主界面自定义背景（data URL / 颜色 / 空） */
  appBackground: string
  /** 登录页自定义背景（data URL / 颜色 / 空） */
  loginBackground: string
}

/** 内置强调色（色相间隔足够大，缩略色块一眼可辨） */
export const PRESET_SEEDS: Record<BuiltinPreset, BrandSeed> = {
  flame: { hue: 40, saturation: 72, lightness: 66 },
  aurora: { hue: 175, saturation: 56, lightness: 66 },
  infinity: { hue: 265, saturation: 62, lightness: 58 },
  ocean: { hue: 240, saturation: 64, lightness: 60 },
  violet: { hue: 300, saturation: 58, lightness: 60 },
  rose: { hue: 10, saturation: 72, lightness: 62 },
  emerald: { hue: 150, saturation: 62, lightness: 62 },
  graphite: { hue: 250, saturation: 8, lightness: 52 },
}
export const BUILTIN_PRESETS = Object.keys(PRESET_SEEDS) as BuiltinPreset[]

export const PRESET_META: Record<ThemePreset, { labelKey: string; descKey: string }> = {
  flame: { labelKey: 'settingsTheme.flame', descKey: 'settingsTheme.flameDesc' },
  aurora: { labelKey: 'settingsTheme.aurora', descKey: 'settingsTheme.auroraDesc' },
  infinity: { labelKey: 'settingsTheme.infinity', descKey: 'settingsTheme.infinityDesc' },
  ocean: { labelKey: 'settingsTheme.ocean', descKey: 'settingsTheme.oceanDesc' },
  violet: { labelKey: 'settingsTheme.violet', descKey: 'settingsTheme.violetDesc' },
  rose: { labelKey: 'settingsTheme.rose', descKey: 'settingsTheme.roseDesc' },
  emerald: { labelKey: 'settingsTheme.emerald', descKey: 'settingsTheme.emeraldDesc' },
  graphite: { labelKey: 'settingsTheme.graphite', descKey: 'settingsTheme.graphiteDesc' },
  custom: { labelKey: 'settingsTheme.custom', descKey: 'settingsTheme.customDesc' },
}
const ALL_PRESETS = new Set<string>(Object.keys(PRESET_META))

export const DEFAULT_THEME: CustomTheme = {
  ...PRESET_SEEDS.flame,
  glassBlur: 14,
  radius: 'standard',
  density: 'standard',
  appBackground: '',
  loginBackground: '',
}

/** 种子 → OKLCH 三元组（暗色下提亮一档，保证深底上的可读性） */
export function seedToOklch(seed: BrandSeed, dark: boolean): { l: number; c: number; h: number } {
  const c = (clamp(seed.saturation, 0, 100) / 100) * 0.25
  const base = clamp(seed.lightness, 25, 85) / 100
  const l = dark ? Math.min(base + 0.06, 0.84) : base
  return { l: round(l), c: round(c), h: Math.round(clamp(seed.hue, 0, 360)) }
}

/** 预设色块的静态预览色（与当前主题无关——旧版色块全部读当前品牌色，看起来一模一样） */
/** 预设的纯色（选中外圈 / 需要单色的场合；渐变不能用作 box-shadow 颜色） */
export function presetColor(id: BuiltinPreset): string {
  const { l, c, h } = seedToOklch(PRESET_SEEDS[id], false)
  return `oklch(${l} ${c} ${h})`
}

export function presetSwatch(id: BuiltinPreset): string {
  const { l, c, h } = seedToOklch(PRESET_SEEDS[id], false)
  return `linear-gradient(135deg, oklch(${round(l + 0.08)} ${c} ${h}), oklch(${round(l - 0.1)} ${c} ${h}))`
}

export const useThemeStore = defineStore('theme', () => {
  const mode = useStorage<ThemeMode>(STORAGE_KEYS.mode, 'dark', undefined, {
    serializer: rawStringSerializer<ThemeMode>(),
  })
  if (!THEME_MODES.includes(mode.value)) mode.value = 'dark'
  const preset = useStorage<ThemePreset>(STORAGE_KEYS.preset, 'flame', undefined, {
    serializer: rawStringSerializer<ThemePreset>(),
  })
  if (!ALL_PRESETS.has(preset.value)) preset.value = 'flame'
  const custom = useStorage<CustomTheme>(STORAGE_KEYS.custom, DEFAULT_THEME, undefined, {
    mergeDefaults: true,
  })
  /** 历史布尔开关（v0.9 之前）：仅用于向 `material` 迁移，保留 key 以兼容旧导出文件 */
  const legacyGlass = useStorage<boolean>(STORAGE_KEYS.glass, true)
  const material = useStorage<ThemeMaterial>(
    STORAGE_KEYS.material,
    legacyGlass.value ? 'glass' : 'solid',
    undefined,
    { serializer: rawStringSerializer<ThemeMaterial>() },
  )
  if (!THEME_MATERIALS.includes(material.value)) material.value = 'glass'

  // ── 明暗：auto 跟随系统 ──
  const systemDark = ref(prefersDark())
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = (e: MediaQueryListEvent) => (systemDark.value = e.matches)
    mql.addEventListener?.('change', onChange)
  }
  const isDark = computed(() => (mode.value === 'auto' ? systemDark.value : mode.value === 'dark'))

  /** 当前生效的品牌种子：内置预设取预设色，自定义取用户值 */
  const seed = computed<BrandSeed>(() => {
    if (preset.value === 'custom') {
      const { hue, saturation, lightness } = custom.value
      return { hue, saturation, lightness }
    }
    return PRESET_SEEDS[preset.value as BuiltinPreset] ?? PRESET_SEEDS.flame
  })

  /** 玻璃总开关（兼容视图）：`material !== 'solid'`；写入等价于在 solid / glass 间切换 */
  const glassEnabled = computed(() => material.value !== 'solid')

  function setMaterial(next: ThemeMaterial) {
    if (!THEME_MATERIALS.includes(next)) return
    material.value = next
    legacyGlass.value = next !== 'solid'
  }

  function setGlassEnabled(next: boolean) {
    setMaterial(next ? 'glass' : 'solid')
  }

  function apply() {
    const root = document.documentElement
    root.classList.toggle('dark', isDark.value)
    root.dataset.material = material.value
    root.classList.toggle('glass-disabled', material.value === 'solid')
    root.setAttribute('data-theme', preset.value)

    const vars: Record<string, string> = {}
    const { l, c, h } = seedToOklch(seed.value, isDark.value)
    // 只写种子三元组：tokens.css 用它派生整条色阶（hover / active / 前景 / ring / 50–900），
    // OpenVue 的 primary 色板也桥接到同一组令牌（theme/flame-preset.ts），组件随之换色
    vars['--fp-brand-l'] = String(l)
    vars['--fp-brand-c'] = String(c)
    vars['--fp-brand-h'] = String(h)

    const eff = custom.value
    vars['--fp-glass-blur'] = `${clamp(eff.glassBlur ?? DEFAULT_THEME.glassBlur, 0, 40)}px`

    const radius = RADII[eff.radius] ?? RADII.standard
    vars['--fp-radius-sm'] = radius.sm
    vars['--fp-radius-md'] = radius.md
    vars['--fp-radius-lg'] = radius.lg
    root.dataset.density = eff.density in DENSITY ? eff.density : 'standard'
    const densityScale = DENSITY[eff.density] ?? DENSITY.standard
    for (const k of Object.keys(densityScale)) {
      vars[`--fp-space-${k}`] = densityScale[k]
    }

    // 自定义背景（背景图优先，颜色回退；背景为本地用户设置，与预设无关）
    const bg = (value: string) =>
      value.startsWith('data:') || value.startsWith('http')
        ? `url("${value}") center/cover no-repeat fixed`
        : value
    vars['--fp-app-bg'] = eff.appBackground ? bg(eff.appBackground) : 'none'
    vars['--fp-login-bg'] = eff.loginBackground ? bg(eff.loginBackground) : 'none'
    root.classList.toggle('app-bg-enabled', !!eff.appBackground)

    // 清理旧版直接写死的派生色（它们会覆盖 tokens.css 的相对色派生，导致暗色 / 预设不跟随）
    for (const legacy of ['--fp-brand', '--fp-brand-strong', '--fp-brand-soft']) {
      root.style.removeProperty(legacy)
    }
    const style = root.style
    for (const [k, v] of Object.entries(vars)) style.setProperty(k, v)
  }

  function setMode(next: ThemeMode) {
    if (THEME_MODES.includes(next)) mode.value = next
  }

  /** 明暗快速切换（顶栏 / ⌘K）：从当前实际外观翻转为显式模式 */
  function toggleDark() {
    mode.value = isDark.value ? 'light' : 'dark'
  }

  function setPreset(next: ThemePreset) {
    if (!ALL_PRESETS.has(next)) return
    if (next === 'custom' && preset.value !== 'custom') {
      // 从当前预设色起步，避免切到「自定义」瞬间跳色
      custom.value = { ...custom.value, ...seed.value }
    }
    preset.value = next
  }

  /** 直接写自定义字段（导入 / 背景 / 形状等偏好；不改变预设） */
  function updateCustom(patch: Partial<CustomTheme>) {
    custom.value = { ...custom.value, ...patch }
  }

  /** 调品牌色：非自定义预设下以当前色为起点切到 custom，保证滑块立刻可见 */
  function tweakBrand(patch: Partial<BrandSeed>) {
    const base = preset.value === 'custom' ? {} : seed.value
    custom.value = { ...custom.value, ...base, ...patch }
    preset.value = 'custom'
  }

  /** 恢复默认外观：强调色、材质、形状（保留背景图——背景属于内容而非主题） */
  function resetCustom() {
    const { appBackground, loginBackground } = custom.value
    custom.value = { ...DEFAULT_THEME, appBackground, loginBackground }
    preset.value = 'flame'
    setMaterial('glass')
  }

  /** 从后端设置同步背景（登录后调用；空值表示使用本地） */
  function syncFromServer(map: Record<string, string>) {
    const patch: Partial<CustomTheme> = {}
    if (map['app_background']) patch.appBackground = map['app_background']
    if (map['login_background']) patch.loginBackground = map['login_background']
    if (Object.keys(patch).length) updateCustom(patch)
  }

  watch([mode, preset, custom, material, isDark], () => apply(), { deep: true })

  return {
    mode,
    isDark,
    preset,
    custom,
    seed,
    material,
    glassEnabled,
    apply,
    setMode,
    toggleDark,
    setPreset,
    setMaterial,
    setGlassEnabled,
    updateCustom,
    tweakBrand,
    resetCustom,
    syncFromServer,
  }
})

function prefersDark(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!window.matchMedia &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  )
}

function clamp(v: number, lo: number, hi: number): number {
  const n = Number.isFinite(v) ? v : lo
  return Math.min(hi, Math.max(lo, n))
}

function round(v: number): number {
  return Math.round(v * 1000) / 1000
}

const RADII: Record<RadiusScale, { sm: string; md: string; lg: string }> = {
  sharp: { sm: '3px', md: '5px', lg: '8px' },
  standard: { sm: '6px', md: '10px', lg: '14px' },
  rounded: { sm: '10px', md: '14px', lg: '20px' },
}

const DENSITY: Record<DensityScale, Record<string, string>> = {
  compact: {
    '1': '2px',
    '2': '6px',
    '3': '8px',
    '4': '12px',
    '5': '16px',
    '6': '20px',
    '8': '24px',
    '10': '32px',
  },
  standard: {
    '1': '4px',
    '2': '8px',
    '3': '12px',
    '4': '16px',
    '5': '20px',
    '6': '24px',
    '8': '32px',
    '10': '40px',
  },
  comfortable: {
    '1': '6px',
    '2': '10px',
    '3': '14px',
    '4': '20px',
    '5': '24px',
    '6': '30px',
    '8': '40px',
    '10': '52px',
  },
}
