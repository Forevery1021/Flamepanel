import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import TwoFactorPanel from '../TwoFactorPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const api = vi.hoisted(() => ({
  fetchTwoFactorStatus: vi.fn(),
  beginTwoFactorSetup: vi.fn(),
  enableTwoFactor: vi.fn(),
  disableTwoFactor: vi.fn(),
  regenerateRecoveryCodes: vi.fn(),
}))
vi.mock('@/api/auth', () => api)

/** 弹窗内联渲染（FpModal 默认 Teleport 到 body，测试里直接展开插槽） */
const FpModalStub = {
  props: ['modelValue', 'header'],
  template:
    '<div v-if="modelValue" class="modal-stub"><h4>{{ header }}</h4><slot /><slot name="footer" /></div>',
}

function mountPanel() {
  return mount(TwoFactorPanel, {
    global: {
      plugins: [createPinia(), createTestI18n({}), openVuePlugin, ...services],
      stubs: { FpModal: FpModalStub, CopyButton: true },
    },
  })
}

const disabled = { enabled: false, pending: false, recovery_codes_remaining: 0, required: false }
const enabled = {
  enabled: true,
  pending: false,
  recovery_codes_remaining: 10,
  enabled_at: '2026-09-28T00:00:00Z',
  required: false,
}

function button(w: ReturnType<typeof mountPanel>, key: string) {
  const b = w.findAll('button').find((x) => x.text().includes(key))
  if (!b) throw new Error(`button ${key} not found: ${w.text()}`)
  return b
}

beforeEach(() => {
  setActivePinia(createPinia())
  Object.values(api).forEach((f) => f.mockReset())
})

describe('TwoFactorPanel', () => {
  it('启用流程：确认密码 → 显示密钥与 otpauth 链接 → 验证码启用 → 一次性展示恢复码', async () => {
    api.fetchTwoFactorStatus.mockResolvedValueOnce({ data: disabled })
    api.beginTwoFactorSetup.mockResolvedValue({
      data: {
        secret: 'JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP',
        otpauth_uri: 'otpauth://totp/x?secret=J',
      },
    })
    api.enableTwoFactor.mockResolvedValue({
      data: { recovery_codes: ['AAAAA-BBBBB', 'CCCCC-DDDDD'] },
    })
    const w = mountPanel()
    await flushPromises()
    expect(w.text()).toContain('twoFactor.statusDisabled')

    await button(w, 'twoFactor.enable').trigger('click')
    await w.find('.modal-stub input[type="password"]').setValue('pw')
    await button(w, 'twoFactor.next').trigger('click')
    await flushPromises()
    expect(api.beginTwoFactorSetup).toHaveBeenCalledWith('pw')
    expect(w.find('.tf-link').attributes('href')).toBe('otpauth://totp/x?secret=J')
    expect(w.find('.tf-secret__value').text()).toBe('JBSW Y3DP EHPK 3PXP JBSW Y3DP EHPK 3PXP')

    // 非 6 位数字时不可提交
    const input = w.find('.modal-stub input[autocomplete="one-time-code"]')
    await input.setValue('12')
    expect(button(w, 'twoFactor.confirmEnable').attributes('disabled')).toBeDefined()
    await input.setValue('123 456')
    await button(w, 'twoFactor.confirmEnable').trigger('click')
    await flushPromises()
    expect(api.enableTwoFactor).toHaveBeenCalledWith('123456')
    expect(w.findAll('.tf-codes li').map((l) => l.text())).toEqual(['AAAAA-BBBBB', 'CCCCC-DDDDD'])
    expect(w.emitted('enabled')).toHaveLength(1)

    api.fetchTwoFactorStatus.mockResolvedValueOnce({ data: enabled })
    await button(w, 'twoFactor.saved').trigger('click')
    await flushPromises()
    expect(w.find('.tf-codes').exists()).toBe(false)
    expect(w.text()).toContain('twoFactor.statusEnabled')
  })

  it('验证码错误（400）显示在输入框下而非全局报错', async () => {
    api.fetchTwoFactorStatus.mockResolvedValue({ data: disabled })
    api.beginTwoFactorSetup.mockResolvedValue({
      data: { secret: 'ABCD', otpauth_uri: 'otpauth://x' },
    })
    api.enableTwoFactor.mockRejectedValue({ status: 400, message: 'invalid verification code' })
    const w = mountPanel()
    await flushPromises()
    await button(w, 'twoFactor.enable').trigger('click')
    await w.find('.modal-stub input[type="password"]').setValue('pw')
    await button(w, 'twoFactor.next').trigger('click')
    await flushPromises()
    await w.find('.modal-stub input[autocomplete="one-time-code"]').setValue('000000')
    await button(w, 'twoFactor.confirmEnable').trigger('click')
    await flushPromises()
    expect(w.text()).toContain('twoFactor.loginInvalid')
  })

  it('已启用：停用需要密码 + 验证码；面板强制时不显示停用按钮', async () => {
    api.fetchTwoFactorStatus.mockResolvedValueOnce({ data: enabled })
    api.disableTwoFactor.mockResolvedValue({ data: { message: 'ok' } })
    const w = mountPanel()
    await flushPromises()
    expect(w.text()).toContain('twoFactor.recoveryRemaining')
    await button(w, 'twoFactor.disable').trigger('click')
    const inputs = w.findAll('.modal-stub input')
    await inputs[0].setValue('pw')
    await inputs[1].setValue('AAAAA-BBBBB')
    api.fetchTwoFactorStatus.mockResolvedValueOnce({ data: disabled })
    await w.findAll('.modal-stub button').at(-1)!.trigger('click')
    await flushPromises()
    expect(api.disableTwoFactor).toHaveBeenCalledWith('pw', 'AAAAA-BBBBB')
    expect(w.text()).toContain('twoFactor.statusDisabled')

    api.fetchTwoFactorStatus.mockResolvedValueOnce({ data: { ...enabled, required: true } })
    const w2 = mountPanel()
    await flushPromises()
    expect(w2.text()).toContain('twoFactor.requiredHint')
    expect(w2.findAll('button').some((b) => b.text() === 'twoFactor.disable')).toBe(false)
  })
})
