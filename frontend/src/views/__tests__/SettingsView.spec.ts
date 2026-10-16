import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import SettingsView from '@/views/SettingsView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * UX-3 面板设置（一级菜单 + 左侧二级导航）的三条新契约：
 *
 * ① **URL 可寻址**：`#security` 直达对应分区；切换分区用 `router.replace` 同步 hash；
 * ② **脏状态感知**：表单与服务端快照 diff——未改动时保存按钮禁用、改动后导航项出现圆点；
 * ③ **原子批量保存**：只把变更键交给 `updateSettingsBatch`（一次事务），
 *    旧实现 7 次串行 `updateSetting` 中途失败会留下半套配置。
 *
 * DNS / 主题两个子组件各有 spec，这里打桩。
 */

const { api, routerMock } = vi.hoisted(() => ({
  api: {
    listSettings: vi.fn(),
    updateSetting: vi.fn(),
    updateSettingsBatch: vi.fn(),
    getAccessInfo: vi.fn(),
    rotateJwtSecret: vi.fn(),
  },
  routerMock: { replace: vi.fn(() => Promise.resolve()), hash: '' },
}))

vi.mock('@/api/settings', () => ({
  listSettings: api.listSettings,
  updateSetting: api.updateSetting,
  updateSettingsBatch: api.updateSettingsBatch,
  getAccessInfo: api.getAccessInfo,
}))
vi.mock('@/api/auth', () => ({
  changePassword: vi.fn(),
  rotateJwtSecret: api.rotateJwtSecret,
  logout: vi.fn(() => Promise.resolve()),
}))
vi.mock('@/api/health', () => ({
  fetchHealthDetail: vi.fn(() =>
    Promise.resolve({ data: { version: '1.2.3', uptime_secs: 90061 } }),
  ),
}))
vi.mock('vue-router', () => ({
  useRoute: () => ({ hash: routerMock.hash, path: '/settings' }),
  useRouter: () => ({ replace: routerMock.replace, push: vi.fn() }),
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('@/locales', () => ({ setLanguage: vi.fn() }))

const messages = {
  nav: {
    settings: '面板设置',
    groupWeb: 'Web',
    groupApps: '应用',
    groupStorage: '存储',
    groupOps: '运维',
    groupSystem: '系统',
  },
  pageDesc: { settings: '配置面板' },
  dnsProvider: { title: 'DNS' },
  settingsTheme: { title: '主题' },
  settingsAppearance: {
    title: '外观',
    interface: '界面',
    menuAccordion: '手风琴',
    menuAccordionHint: '',
    menuShow: '隐藏分组',
    menuShowHint: '',
    background: '背景',
    appBackground: '主界面背景',
    loginBackground: '登录页背景',
    upload: '上传',
    clear: '清除',
    bgHint: '',
    onboardingHint: '',
  },
  onboarding: { replay: '重新播放引导' },
  settings: {
    title: '面板设置',
    panelConfig: '面板配置',
    security: '安全',
    backupSettings: '自动备份设置',
    panelName: '面板名称',
    language: '语言',
    panelPort: '端口',
    sessionTimeout: '会话超时',
    logLevel: '日志级别',
    logRetention: '日志保留',
    twoFactor: '两步验证',
    accessCurrent: 'IP {ip} · {host}',
    save: '保存',
    unsaved: '有未保存的更改',
    unsavedHint: '更改尚未保存',
    nothingToSave: '没有需要保存的更改',
    leaveConfirm: '离开？',
    discard: '放弃',
    changePassword: '修改密码',
    oldPassword: '旧',
    newPassword: '新',
    confirmPassword: '确认',
    jwtSecret: '密钥',
    alreadySet: '已设置',
    rotate: '轮换',
    autoBackup: '自动备份',
    autoBackupHint: '',
    backupInterval: '间隔',
    hoursUnit: '小时',
    backupRetention: '保留',
    backupRetentionHint: '',
    sectionDesc: {
      general: 'g',
      security: 's',
      backup: 'b',
      dns: 'd',
      theme: 't',
      appearance: 'a',
    },
  },
  common: { reset: '重置', success: '成功', failed: '失败', confirmAction: '确认', cancel: '取消' },
}

const serverSettings = [
  { key: 'panel_name', value: 'FlamePanel' },
  { key: 'language', value: 'zh-CN' },
  { key: 'panel_port', value: '8080' },
  { key: 'session_timeout_minutes', value: '1440' },
  { key: 'log_level', value: 'info' },
  { key: 'log_retention_days', value: '30' },
  { key: 'allowed_ips', value: '10.0.0.0/8,192.168.1.1' },
  { key: 'auto_backup_enabled', value: 'false' },
  { key: 'auto_backup_interval_hours', value: '24' },
  { key: 'backup_retention', value: '7' },
]

function mountView() {
  return mount(SettingsView, {
    global: {
      plugins: [createPinia(), createTestI18n(messages), openVuePlugin, ...services],
      stubs: {
        SettingsDnsTab: { template: '<div class="stub-dns" />' },
        SettingsThemeTab: { template: '<div class="stub-theme" />' },
        LayoutContent: { template: '<div><slot /></div>' },
      },
    },
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  setActivePinia(createPinia())
  routerMock.hash = ''
  api.listSettings.mockResolvedValue({
    data: { data: serverSettings, total: serverSettings.length },
  })
  api.updateSettingsBatch.mockResolvedValue({ data: null })
  api.getAccessInfo.mockResolvedValue({
    data: {
      client_ip: '203.0.113.9',
      host: 'panel.example.invalid',
      local: false,
      allowed_ips: [],
      bind_domains: [],
      session_timeout_minutes: 1440,
      login_max_failures: 5,
      login_lockout_minutes: 5,
    },
  })
})

describe('SettingsView · 二级导航与 URL', () => {
  it('默认落在「面板配置」，七个分区全部可见', async () => {
    const w = mountView()
    await flushPromises()
    expect(w.findAll('[role="tab"]')).toHaveLength(7)
    expect(w.find('#settings-tab-general').classes()).toContain('is-active')
    expect(w.find('#settings-panel-general').exists()).toBe(true)
  })

  it('URL hash 深链直达对应分区（#security）', async () => {
    routerMock.hash = '#security'
    const w = mountView()
    await flushPromises()
    expect(w.find('#settings-tab-security').classes()).toContain('is-active')
    expect(w.find('#settings-panel-security').exists()).toBe(true)
  })

  it('未知 hash 回落到 general', async () => {
    routerMock.hash = '#nope'
    const w = mountView()
    await flushPromises()
    expect(w.find('#settings-tab-general').classes()).toContain('is-active')
  })

  it('点击分区 → 内容切换并用 replace 同步 hash（不产生历史记录）', async () => {
    const w = mountView()
    await flushPromises()
    await w.find('#settings-tab-dns').trigger('click')
    expect(w.find('.stub-dns').exists()).toBe(true)
    expect(routerMock.replace).toHaveBeenCalledWith({ hash: '#dns' })
  })

  it('键盘：↓ 切到下一分区，End 到最后一个', async () => {
    const w = mountView()
    await flushPromises()
    await w.find('#settings-tab-general').trigger('keydown', { key: 'ArrowDown' })
    expect(w.find('#settings-tab-security').classes()).toContain('is-active')
    await w.find('#settings-tab-security').trigger('keydown', { key: 'End' })
    expect(w.find('#settings-tab-about').classes()).toContain('is-active')
  })
})

describe('SettingsView · 脏状态与原子保存', () => {
  it('加载后无改动：保存按钮禁用、导航项无未保存圆点', async () => {
    const w = mountView()
    await flushPromises()
    const save = w.find('#settings-panel-general .settings-actions button:first-child')
    expect(save.attributes('disabled')).toBeDefined()
    expect(w.find('.settings-nav__dot').exists()).toBe(false)
  })

  it('修改面板名称 → 出现圆点、保存可用；保存只发送变更键并走批量接口', async () => {
    const w = mountView()
    await flushPromises()
    const input = w.find('#settings-panel-general input')
    await input.setValue('My Panel')
    await nextTick()
    expect(w.find('#settings-tab-general .settings-nav__dot').exists()).toBe(true)
    const save = w.find('#settings-panel-general .settings-actions button:first-child')
    expect(save.attributes('disabled')).toBeUndefined()

    await save.trigger('click')
    await flushPromises()
    expect(api.updateSettingsBatch).toHaveBeenCalledTimes(1)
    expect(api.updateSettingsBatch).toHaveBeenCalledWith([['panel_name', 'My Panel']])
    // 旧实现的逐键 PUT 不应再被调用
    expect(api.updateSetting).not.toHaveBeenCalled()
    // 保存成功后本地快照就地更新：脏标记消失、按钮再次禁用
    await nextTick()
    expect(w.find('.settings-nav__dot').exists()).toBe(false)
    expect(
      w.find('#settings-panel-general .settings-actions button:first-child').attributes('disabled'),
    ).toBeDefined()
  })

  it('重置把表单恢复为服务端快照', async () => {
    const w = mountView()
    await flushPromises()
    const input = w.find('#settings-panel-general input')
    await input.setValue('Changed')
    await nextTick()
    const reset = w.findAll('#settings-panel-general .settings-actions button')[1]
    await reset.trigger('click')
    await nextTick()
    expect((w.find('#settings-panel-general input').element as HTMLInputElement).value).toBe(
      'FlamePanel',
    )
    expect(w.find('.settings-nav__dot').exists()).toBe(false)
  })
})

describe('SettingsView · 安全分区（W4-a）', () => {
  it('进入安全分区拉取访问来源；列表设置按规范形比较（换行 ≡ 逗号，不算改动）', async () => {
    routerMock.hash = '#security'
    const w = mountView()
    await flushPromises()
    expect(api.getAccessInfo).toHaveBeenCalledTimes(1)
    expect(w.text()).toContain('203.0.113.9')
    const ta = w.find('#settings-panel-security textarea')
    // 服务端存逗号分隔，表单按行展示
    expect((ta.element as HTMLTextAreaElement).value).toBe('10.0.0.0/8\n192.168.1.1')
    expect(w.find('#settings-tab-security .settings-nav__dot').exists()).toBe(false)

    await ta.setValue('10.0.0.0/8\n192.168.1.1\n203.0.113.9')
    await nextTick()
    expect(w.find('#settings-tab-security .settings-nav__dot').exists()).toBe(true)
    await w.find('#settings-panel-security .settings-actions button:first-child').trigger('click')
    await flushPromises()
    expect(api.updateSettingsBatch).toHaveBeenCalledWith([
      ['allowed_ips', '10.0.0.0/8,192.168.1.1,203.0.113.9'],
    ])
  })

  it('端口只读展示，不进入保存键集合', async () => {
    const w = mountView()
    await flushPromises()
    expect(w.find('#settings-panel-general').text()).toContain('8080')
    const input = w.find('#settings-panel-general input')
    await input.setValue('X')
    await nextTick()
    await w.find('#settings-panel-general .settings-actions button:first-child').trigger('click')
    await flushPromises()
    const sent = api.updateSettingsBatch.mock.calls[0][0] as [string, string][]
    expect(sent.map(([k]) => k)).toEqual(['panel_name'])
  })

  it('关于分区展示版本与运行时长', async () => {
    routerMock.hash = '#about'
    const w = mountView()
    await flushPromises()
    expect(w.find('#settings-panel-about').text()).toContain('v1.2.3')
  })
})
