import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SettingsAppearanceTab from '../SettingsAppearanceTab.vue'
import { useAppearanceStore } from '@/stores/appearance'
import { useThemeStore } from '@/stores/theme'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const updateSetting = vi.fn()
vi.mock('@/api/settings', () => ({
  updateSetting: (...a: unknown[]) => updateSetting(...a),
}))
vi.mock('@/locales', () => ({
  setLanguage: vi.fn(),
  i18n: { global: { locale: { value: 'zh-CN' } } },
}))

/** 界面分区（从 SettingsView 拆出）：改动即写 store 并单键持久化，不经表单批量保存。 */
let pinia: ReturnType<typeof createPinia>
function mountTab() {
  return mount(SettingsAppearanceTab, {
    global: { plugins: [pinia, createTestI18n({}), openVuePlugin, ...services] },
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  pinia = createPinia()
  setActivePinia(pinia)
  updateSetting.mockResolvedValue({ data: null })
})

describe('SettingsAppearanceTab', () => {
  it('切换菜单手风琴：更新 store 并持久化 menu_accordion', async () => {
    const w = mountTab()
    await w.find('input[type="checkbox"]').setValue(true)
    await flushPromises()
    expect(useAppearanceStore().state.menuAccordion).toBe(true)
    expect(updateSetting).toHaveBeenCalledWith('menu_accordion', 'true')
  })

  it('已设置背景时可清除：store 置空并持久化空值', async () => {
    const theme = useThemeStore()
    theme.updateCustom({ loginBackground: 'data:image/jpeg;base64,AAAA' })
    const w = mountTab()
    expect(w.findAll('.bg-preview')).toHaveLength(1)
    const clear = w.findAll('button').find((b) => b.text() === 'settingsAppearance.clear')
    expect(clear).toBeTruthy()
    await clear!.trigger('click')
    await flushPromises()
    expect(theme.custom.loginBackground).toBe('')
    expect(updateSetting).toHaveBeenCalledWith('login_background', '')
  })
})
