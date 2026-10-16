import { describe, it, expect } from 'vitest'
import { formatLabel, modeLabel, modeTagType, isNativeMode } from '@/utils/appStoreFormat'

/**
 * T2-4 拆分伴生的纯函数单测（`AppStoreView` → `utils/appStoreFormat.ts`）。
 *
 * 这三个映射原先内联在 1464 行的视图里、被卡片网格 / 已安装表 / 模式下拉三处
 * 各写一份；抽为纯函数后可机器验证「未知值原样透出」（历史语义，不得默默改写
 * 成看似合理的默认值——与 R-9 的立场一致）。
 */

describe('formatLabel（应用包格式）', () => {
  it('已知格式 → 品牌名', () => {
    expect(formatLabel('flame')).toBe('Flame')
    expect(formatLabel('onepanel')).toBe('1Panel')
    expect(formatLabel('baota')).toBe('宝塔')
  })

  it('未知格式原样透出（不伪造映射）', () => {
    expect(formatLabel('mystery')).toBe('mystery')
    expect(formatLabel('')).toBe('')
  })
})

describe('modeLabel（安装模式文案）', () => {
  const t = (key: string) => `t:${key}`

  it('container / native 走 i18n；wasm 为技术名词不翻译', () => {
    expect(modeLabel('container', t)).toBe('t:appStore.modeContainer')
    expect(modeLabel('native', t)).toBe('t:appStore.modeNative')
    expect(modeLabel('wasm', t)).toBe('WASM')
  })

  it('未知模式原样透出（不默认展示成某个已知模式）', () => {
    expect(modeLabel('unknown-mode', t)).toBe('unknown-mode')
  })
})

describe('modeTagType（标签配色）', () => {
  it('wasm 警告色 / native 成功色 / 其余信息色', () => {
    expect(modeTagType('wasm')).toBe('warning')
    expect(modeTagType('native')).toBe('success')
    expect(modeTagType('container')).toBe('info')
  })

  it('未知模式回落信息色（视觉默认档，不改写语义）', () => {
    expect(modeTagType('whatever')).toBe('info')
  })
})

describe('isNativeMode（生命周期操作入口条件）', () => {
  it('仅 native 模式为真——启停按钮只对该模式渲染', () => {
    expect(isNativeMode({ mode: 'native' })).toBe(true)
    expect(isNativeMode({ mode: 'container' })).toBe(false)
    expect(isNativeMode({ mode: 'wasm' })).toBe(false)
  })

  it('空值容错（列表早期快照 / 缺字段不抛错）', () => {
    expect(isNativeMode(null)).toBe(false)
    expect(isNativeMode(undefined)).toBe(false)
    expect(isNativeMode({})).toBe(false)
    expect(isNativeMode({ mode: null })).toBe(false)
  })
})
