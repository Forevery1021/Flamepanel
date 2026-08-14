import { defineStore } from 'pinia'
import { watch } from 'vue'
import { useStorage } from '@vueuse/core'
import { STORAGE_KEYS, rawStringSerializer } from '@/utils/storage'

export type ThemeMode = 'light' | 'dark'
export type ThemePreset = 'flame' | 'aurora' | 'infinity' | 'custom'
export type RadiusScale = 'sharp' | 'standard' | 'rounded'
export type DensityScale = 'compact' | 'standard' | 'comfortable'

export interface CustomTheme {
  hue: number
  saturation: number
  lightness: number
  glassBlur: number
  radius: RadiusScale
  density: DensityScale
  /** 主界面自定义背景（data URL / 颜色 / 空） */
  appBackground: string
  /** 登录页自定义背景（data URL / 颜色 / 空） */
  loginBackground: string
}

export const PRESET_META: Record<ThemePreset, { labelKey: string; descKey: string }> = {
  flame: { labelKey: 'settingsTheme.flame', descKey: 'settingsTheme.flameDesc' },
  aurora: { labelKey: 'settingsTheme.aurora', descKey: 'settingsTheme.auroraDesc' },
  infinity: { labelKey: 'settingsTheme.infinity', descKey: 'settingsTheme.infinityDesc' },
  custom: { labelKey: 'settingsTheme.custom', descKey: 'settingsTheme.customDesc' },
}

const DEFAULT_THEME: CustomTheme = {
  hue: 35,
  saturation: 65,
  lightness: 62,
  glassBlur: 12,
  radius: 'standard',
  density: 'standard',
  appBackground: '',
  loginBackground: '',
}

export const useThemeStore = defineStore('theme', () => {
  // P6：改用 @vueuse/core useStorage 统一持久化（保持既有存储格式，避免漂移）
  const mode = useStorage<ThemeMode>(STORAGE_KEYS.mode, 'dark', undefined, {
    serializer: rawStringSerializer<ThemeMode>(),
  })
  const preset = useStorage<ThemePreset>(STORAGE_KEYS.preset, 'flame', undefined, {
    serializer: rawStringSerializer<ThemePreset>(),
  })
  const custom = useStorage<CustomTheme>(STORAGE_KEYS.custom, DEFAULT_THEME, undefined, {
    mergeDefaults: true,
  })
  /** 玻璃总开关：关闭时全部回退实色表面（低端设备 / 用户偏好） */
  const glassEnabled = useStorage<boolean>(STORAGE_KEYS.glass, true)

  function setGlassEnabled(next: boolean) {
    glassEnabled.value = next
    document.documentElement.classList.toggle('glass-disabled', !next)
  }

  function apply() {
    const root = document.documentElement
    root.classList.toggle('dark', mode.value === 'dark')
    root.classList.toggle('glass-disabled', !glassEnabled.value)
    root.setAttribute('data-theme', preset.value)

    const vars: Record<string, string> = {}
    const isCustom = preset.value === 'custom'
    const eff = isCustom
      ? custom.value
      : (DEFAULT_CUSTOM[preset.value as Exclude<ThemePreset, 'custom'>] ?? DEFAULT_CUSTOM.flame)

    // 品牌色（OKLCH 由 HSL 参数生成）
    const chroma = (eff.saturation / 100) * 0.25
    const lightness = eff.lightness / 100
    const brand = `oklch(${lightness} ${chroma} ${eff.hue})`
    vars['--fp-brand'] = brand
    vars['--fp-brand-strong'] = `oklch(${Math.max(lightness - 0.08, 0.3)} ${chroma} ${eff.hue})`
    vars['--fp-brand-soft'] = `oklch(${lightness} ${chroma} ${eff.hue} / 0.14)`

    // 玻璃模糊
    vars['--fp-glass-blur'] = `${eff.glassBlur}px`

    // 圆角
    const radius = RADII[eff.radius]
    vars['--fp-radius-sm'] = radius.sm
    vars['--fp-radius-md'] = radius.md
    vars['--fp-radius-lg'] = radius.lg

    // 密度 → 间距比例
    const densityScale = DENSITY[eff.density]
    for (const k of Object.keys(densityScale) as Array<keyof typeof densityScale>) {
      vars[`--fp-space-${k}`] = densityScale[k]
    }

    // 自定义背景（背景图优先，颜色回退；背景为本地用户设置，与预设无关）
    const bg = (value: string) =>
      value.startsWith('data:') || value.startsWith('http')
        ? `url("${value}") center/cover no-repeat fixed`
        : value
    const appBgValue = custom.value.appBackground
    const loginBgValue = custom.value.loginBackground
    vars['--fp-app-bg'] = appBgValue ? bg(appBgValue) : 'none'
    vars['--fp-login-bg'] = loginBgValue ? bg(loginBgValue) : 'none'
    root.classList.toggle('app-bg-enabled', !!appBgValue)

    // 批量写 CSS 变量：一次重排，避免逐条 setProperty 多次强制回流（F2.3）
    const style = root.style
    for (const [k, v] of Object.entries(vars)) {
      style.setProperty(k, v)
    }
  }

  function setMode(next: ThemeMode) {
    mode.value = next
  }

  function setPreset(next: ThemePreset) {
    preset.value = next
  }

  function updateCustom(patch: Partial<CustomTheme>) {
    custom.value = { ...custom.value, ...patch }
  }

  function resetCustom() {
    custom.value = { ...DEFAULT_CUSTOM.flame }
  }

  /** 从后端设置同步背景（登录后调用；空值表示使用本地） */
  function syncFromServer(map: Record<string, string>) {
    const patch: Partial<CustomTheme> = {}
    if (map['app_background']) patch.appBackground = map['app_background']
    if (map['login_background']) patch.loginBackground = map['login_background']
    if (Object.keys(patch).length) updateCustom(patch)
  }

  watch([mode, preset, custom], () => apply(), { deep: true })

  return {
    mode,
    preset,
    custom,
    glassEnabled,
    apply,
    setMode,
    setPreset,
    setGlassEnabled,
    updateCustom,
    resetCustom,
    syncFromServer,
  }
})

const RADII: Record<RadiusScale, { sm: string; md: string; lg: string }> = {
  sharp: { sm: '4px', md: '6px', lg: '8px' },
  standard: { sm: '8px', md: '12px', lg: '16px' },
  rounded: { sm: '12px', md: '16px', lg: '20px' },
}

const DENSITY: Record<DensityScale, Record<string, string>> = {
  compact: {
    '1': '2px',
    '2': '4px',
    '3': '8px',
    '4': '10px',
    '5': '14px',
    '6': '16px',
    '8': '20px',
    '10': '28px',
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
    '2': '12px',
    '3': '16px',
    '4': '20px',
    '5': '28px',
    '6': '32px',
    '8': '40px',
    '10': '56px',
  },
}

const DEFAULT_CUSTOM: Record<Exclude<ThemePreset, 'custom'>, CustomTheme> = {
  flame: { hue: 35, saturation: 65, lightness: 62, glassBlur: 12, radius: 'standard', density: 'standard', appBackground: '', loginBackground: '' },
  aurora: { hue: 35, saturation: 55, lightness: 55, glassBlur: 10, radius: 'rounded', density: 'comfortable', appBackground: '', loginBackground: '' },
  infinity: { hue: 45, saturation: 75, lightness: 68, glassBlur: 0, radius: 'sharp', density: 'compact', appBackground: '', loginBackground: '' },
}
