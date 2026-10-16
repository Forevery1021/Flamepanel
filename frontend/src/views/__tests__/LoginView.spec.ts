import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import LoginView from '../LoginView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const push = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push }),
  useRoute: () => ({ query: {} }),
}))

const api = vi.hoisted(() => ({
  login: vi.fn(),
  loginTwoFactor: vi.fn(),
  fetchMe: vi.fn(),
  logout: vi.fn(),
  changePassword: vi.fn(),
}))
vi.mock('@/api/auth', () => api)

const session = {
  token: 'access',
  refresh_token: '',
  username: 'alice',
  role: 'operator',
  must_change_password: false,
}

function mountLogin() {
  return mount(LoginView, {
    global: { plugins: [createPinia(), createTestI18n({}), openVuePlugin, ...services] },
  })
}

async function submitPassword(w: ReturnType<typeof mountLogin>) {
  const inputs = w.findAll('form input')
  await inputs[0].setValue('alice')
  await inputs[1].setValue('pw')
  await w.find('form').trigger('submit')
  await flushPromises()
}

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  push.mockReset()
  Object.values(api).forEach((f) => f.mockReset())
  api.fetchMe.mockResolvedValue({ data: { id: 2, username: 'alice', role: 'operator' } })
})

describe('LoginView 两步验证', () => {
  it('需要第二因子时进入验证码步骤，不保存任何凭证；验证通过后进入仪表盘', async () => {
    api.login.mockResolvedValue({
      data: { ...session, token: '', two_factor_required: true, challenge: 'mfa-jwt' },
    })
    api.loginTwoFactor.mockResolvedValue({ data: session })
    const w = mountLogin()
    await submitPassword(w)

    expect(w.text()).toContain('twoFactor.loginTitle')
    expect(localStorage.getItem('token')).toBeFalsy()
    expect(push).not.toHaveBeenCalled()

    await w.find('form input').setValue(' 123456 ')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(api.loginTwoFactor).toHaveBeenCalledWith('mfa-jwt', '123456')
    expect(push).toHaveBeenCalledWith('/dashboard')
  })

  it('验证码错误提示在输入框；挑战过期回到密码步骤；可切换恢复码', async () => {
    api.login.mockResolvedValue({
      data: { ...session, token: '', two_factor_required: true, challenge: 'mfa-jwt' },
    })
    const w = mountLogin()
    await submitPassword(w)

    api.loginTwoFactor.mockRejectedValueOnce({
      code: 'AUTH_UNAUTHORIZED',
      message: 'Invalid verification code',
    })
    await w.find('form input').setValue('000000')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(w.text()).toContain('twoFactor.loginInvalid')

    const toggle = w.findAll('button').find((b) => b.text() === 'twoFactor.loginUseRecovery')!
    await toggle.trigger('click')
    expect(w.text()).toContain('twoFactor.loginRecoveryHint')

    api.loginTwoFactor.mockRejectedValueOnce({
      code: 'AUTH_UNAUTHORIZED',
      message: 'Invalid or expired two-factor challenge',
    })
    await w.find('form input').setValue('AAAAA-BBBBB')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(w.text()).not.toContain('twoFactor.loginTitle')
    expect(w.text()).toContain('login.title')
  })

  it('面板强制两步验证而未绑定：登录后进入绑定页', async () => {
    api.login.mockResolvedValue({ data: { ...session, must_setup_two_factor: true } })
    const w = mountLogin()
    await submitPassword(w)
    expect(push).toHaveBeenCalledWith('/two-factor-setup')
  })
})
