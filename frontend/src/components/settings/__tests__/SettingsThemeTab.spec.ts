import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import SettingsThemeTab from '../SettingsThemeTab.vue'
import { useThemeStore, BUILTIN_PRESETS } from '@/stores/theme'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

vi.mock('@/locales', () => ({
  setLanguage: vi.fn(),
  i18n: { global: { locale: { value: 'zh-CN' } } },
}))

/**
 * W4-a 主题页：色块直接驱动 store（点即生效，无需保存）；导入只接受已知字段。
 */
let pinia: ReturnType<typeof createPinia>
function mountTab() {
  return mount(SettingsThemeTab, {
    global: { plugins: [pinia, createTestI18n({}), openVuePlugin, ...services] },
  })
}

beforeEach(() => {
  localStorage.clear()
  pinia = createPinia()
  setActivePinia(pinia)
})

describe('SettingsThemeTab', () => {
  it('每个内置预设一个色块 + 自定义；点击即切换预设', async () => {
    const w = mountTab()
    const swatches = w.findAll('.swatch')
    expect(swatches).toHaveLength(BUILTIN_PRESETS.length + 1)
    await swatches[BUILTIN_PRESETS.indexOf('ocean')].trigger('click')
    await nextTick()
    expect(useThemeStore().preset).toBe('ocean')
    expect(swatches[BUILTIN_PRESETS.indexOf('ocean')].attributes('aria-checked')).toBe('true')
  })

  it('材质卡片切换 data-material', async () => {
    const w = mountTab()
    await w.find('.material--acrylic').trigger('click')
    await nextTick()
    expect(document.documentElement.dataset.material).toBe('acrylic')
  })

  it('导入：已知字段生效，垃圾字段 / 非法值被忽略', () => {
    const w = mountTab()
    const store = useThemeStore()
    ;(w.vm as unknown as { applyImported: (d: unknown) => void }).applyImported({
      theme: {
        mode: 'light',
        preset: 'violet',
        material: 'neon',
        custom: { hue: 123, radius: 'rounded', evil: '<script>', glassBlur: 'x' },
      },
    })
    expect(store.mode).toBe('light')
    expect(store.preset).toBe('violet')
    expect(store.material).toBe('glass')
    expect(store.custom.hue).toBe(123)
    expect(store.custom.radius).toBe('rounded')
    expect((store.custom as unknown as Record<string, unknown>).evil).toBeUndefined()
    expect(store.custom.glassBlur).toBe(14)
  })
})
