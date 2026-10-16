import { describe, it, expect, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { useThemeStore, PRESET_SEEDS, seedToOklch, presetColor } from '@/stores/theme'

/**
 * W4-a 主题 store 契约——对应实机「颜色预设 / 材质设置不生效」的三个根因：
 * ① 预设必须改写品牌种子（--fp-brand-l/c/h），而不是只换一个 data-theme 名字；
 * ② 非自定义预设下拖滑块要切到 custom 并以当前色为起点（否则滑块改了也不读）；
 * ③ 材质写到 <html data-material>，solid 同时挂 glass-disabled（tokens.css 据此切表面令牌）。
 */
const root = () => document.documentElement
const brandH = () => root().style.getPropertyValue('--fp-brand-h')

beforeEach(() => {
  localStorage.clear()
  root().removeAttribute('style')
  root().removeAttribute('data-material')
  root().className = ''
  setActivePinia(createPinia())
})

describe('theme store', () => {
  it('切换预设 → 品牌色相随预设种子变化', async () => {
    const s = useThemeStore()
    s.apply()
    expect(brandH()).toBe(String(PRESET_SEEDS.flame.hue))
    s.setPreset('ocean')
    await nextTick()
    expect(brandH()).toBe(String(PRESET_SEEDS.ocean.hue))
    expect(root().getAttribute('data-theme')).toBe('ocean')
  })

  it('非自定义预设下调色相 → 以当前色为起点切到 custom', async () => {
    const s = useThemeStore()
    s.setPreset('emerald')
    s.tweakBrand({ hue: 200 })
    await nextTick()
    expect(s.preset).toBe('custom')
    expect(s.custom.hue).toBe(200)
    // 其余分量沿用翡翠预设，而不是跳回默认橙色
    expect(s.custom.saturation).toBe(PRESET_SEEDS.emerald.saturation)
    expect(brandH()).toBe('200')
  })

  it('材质写入 data-material；solid 同时挂 glass-disabled', async () => {
    const s = useThemeStore()
    s.setMaterial('acrylic')
    await nextTick()
    expect(root().dataset.material).toBe('acrylic')
    expect(root().classList.contains('glass-disabled')).toBe(false)
    s.setMaterial('solid')
    await nextTick()
    expect(root().dataset.material).toBe('solid')
    expect(root().classList.contains('glass-disabled')).toBe(true)
  })

  it('非法材质 / 预设被忽略', () => {
    const s = useThemeStore()
    s.setMaterial('neon' as never)
    s.setPreset('nope' as never)
    expect(s.material).toBe('glass')
    expect(s.preset).toBe('flame')
  })

  it('toggleDark 从 auto 翻转为显式模式', () => {
    const s = useThemeStore()
    s.setMode('auto')
    const wasDark = s.isDark
    s.toggleDark()
    expect(s.mode).toBe(wasDark ? 'light' : 'dark')
  })

  it('圆角 / 密度在任意预设下都即时生效', async () => {
    const s = useThemeStore()
    s.setPreset('violet')
    s.updateCustom({ radius: 'sharp', density: 'compact' })
    await nextTick()
    expect(root().style.getPropertyValue('--fp-radius-md')).toBe('5px')
    expect(root().dataset.density).toBe('compact')
    expect(s.preset).toBe('violet')
  })

  it('色块与种子同源；暗色下亮度提一档', () => {
    const light = seedToOklch(PRESET_SEEDS.rose, false)
    const dark = seedToOklch(PRESET_SEEDS.rose, true)
    expect(dark.l).toBeGreaterThan(light.l)
    expect(presetColor('rose')).toBe(`oklch(${light.l} ${light.c} ${light.h})`)
  })

  it('resetCustom 保留背景图', () => {
    const s = useThemeStore()
    s.updateCustom({ appBackground: 'data:image/png;base64,AA', hue: 99 })
    s.setMaterial('solid')
    s.resetCustom()
    expect(s.custom.appBackground).toBe('data:image/png;base64,AA')
    expect(s.custom.hue).toBe(PRESET_SEEDS.flame.hue)
    expect(s.material).toBe('glass')
  })
})
