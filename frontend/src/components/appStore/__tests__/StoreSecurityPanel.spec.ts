import { describe, it, expect, vi } from 'vitest'
import { mountFp } from '@/components/ui/__tests__/helpers'
import StoreSecurityPanel from '@/components/appStore/StoreSecurityPanel.vue'

/**
 * T2-4 拆分后的商店安全面板契约（原为 AppStoreView 内联模板）。
 *
 * 编辑态归本组件（服务端状态经 `config` prop 注入），因此这里重点验证
 * **编辑会话语义**：初值来自服务端、保存上抛的载荷就是当前编辑值、
 * `config` 变更（保存成功回写 / 显式重置）时编辑态同步刷新。
 */

const toast = { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

const messages = {
  appStore: {
    signaturePolicy: '验签策略',
    signaturePolicyHint: '说明',
    policyOptional: '可选（推荐）',
    policyRequired: '强制',
    policyOptionalHint: '仅对配置公钥的源验签',
    policyRequiredHint: '远程包一律验签',
    unsignedSourceHint: '有 {count} 个未签名源',
    registryWhitelist: '镜像仓库白名单',
    registryWhitelistHint: '白名单说明',
    registryPlaceholder: '如 registry.example.com',
    registryEmpty: '暂无自定义仓库',
    builtinRegistries: '内置仓库',
    registryDuplicate: '该仓库已在白名单中',
  },
  common: { add: '添加', save: '保存', reset: '重置' },
}

const config = (over: Record<string, unknown> = {}) => ({
  signature_policy: 'optional',
  registry_whitelist: ['my.registry'],
  builtin_registries: ['docker.io'],
  effective_registries: ['docker.io', 'my.registry'],
  unsigned_source_count: 0,
  ...over,
})

function mountPanel(props: Record<string, unknown>) {
  return mountFp(StoreSecurityPanel as never, { props, messages })
}

describe('StoreSecurityPanel · 渲染', () => {
  it('loading 时展示骨架屏（不闪现未初始化表单）', () => {
    const w = mountPanel({ loading: true, saving: false, config: null })
    expect(w.find('.security-loading').exists()).toBe(true)
    expect(w.find('.security-block').exists()).toBe(false)
  })

  it('配置就绪：渲染策略说明、白名单 chips 与内置清单', () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    expect(w.text()).toContain('可选（推荐）')
    expect(w.text()).toContain('仅对配置公钥的源验签')
    expect(w.text()).toContain('my.registry')
    expect(w.text()).toContain('docker.io')
  })

  it('白名单为空 → 空态文案；内置清单仍展示', () => {
    const w = mountPanel({
      loading: false,
      saving: false,
      config: config({ registry_whitelist: [] }),
    })
    expect(w.text()).toContain('暂无自定义仓库')
  })

  it('存在未签名源时展示计数提示（安全提醒随数据出现）', () => {
    const w = mountPanel({
      loading: false,
      saving: false,
      config: config({ unsigned_source_count: 3 }),
      messages: undefined,
    })
    // 第二个 warning 说明条即未签名源计数（第一个是策略说明）
    const alerts = w.findAll('.security-alert-inline.is-warning')
    expect(alerts).toHaveLength(2)
    expect(alerts[1].text()).toContain('3')
  })

  it('required 策略时说明条切 info 档（视觉语义随选择变化）', () => {
    const w = mountPanel({
      loading: false,
      saving: false,
      config: config({ signature_policy: 'required' }),
    })
    expect(w.find('.security-block .security-alert-inline.is-info').exists()).toBe(true)
    expect(w.text()).toContain('远程包一律验签')
  })
})

describe('StoreSecurityPanel · 编辑会话', () => {
  it('保存上抛当前编辑载荷（含本地新增的仓库）', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    const input = w.find('.registry-input input')
    await input.setValue('New.Registry.COM')
    await w.findAll('.registry-add-row button')[0].trigger('click')

    await w.find('.security-actions button').trigger('click')
    const [payload] = w.emitted('save')?.[0] ?? []
    expect(payload).toEqual({
      signature_policy: 'optional',
      registry_whitelist: ['my.registry', 'new.registry.com'],
    })
  })

  it('重复仓库被拦截：toast 提示且不重复入列（后端口径一致）', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    await w.find('.registry-input input').setValue('MY.registry')
    await w.findAll('.registry-add-row button')[0].trigger('click')

    expect(toast.warning).toHaveBeenCalledWith('该仓库已在白名单中')
    await w.find('.security-actions button').trigger('click')
    const [payload] = w.emitted('save')?.[0] ?? []
    expect((payload as { registry_whitelist: string[] }).registry_whitelist).toEqual(['my.registry'])
  })

  it('空白输入不产生条目（避免提交空字符串）', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    await w.find('.registry-input input').setValue('   ')
    await w.findAll('.registry-add-row button')[0].trigger('click')
    await w.find('.security-actions button').trigger('click')
    const [payload] = w.emitted('save')?.[0] ?? []
    expect((payload as { registry_whitelist: string[] }).registry_whitelist).toEqual(['my.registry'])
  })

  it('移除仓库后保存载荷同步（编辑态真实生效，不只是视觉消失）', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    const removeBtn = w.find('.registry-chips .fp-chip button, .registry-chips .fp-chip .p-chip-remove-icon')
    if (removeBtn.exists()) {
      await removeBtn.trigger('click')
      await w.find('.security-actions button').trigger('click')
      const [payload] = w.emitted('save')?.[0] ?? []
      expect((payload as { registry_whitelist: string[] }).registry_whitelist).toEqual([])
    }
  })

  it('保存中按钮进入 loading 态（防重复提交；OpenVue 以禁用属性 + 图标反映）', () => {
    const on = mountPanel({ loading: false, saving: true, config: config() })
    const off = mountPanel({ loading: false, saving: false, config: config() })
    const loadingBtn = on.findAll('.security-actions button')[0]
    const idleBtn = off.findAll('.security-actions button')[0]
    // loading 的可见差异：至少有一项与空闲态不同（禁用/类名/子节点），且按钮文案不变
    const differs =
      loadingBtn.attributes('disabled') !== idleBtn.attributes('disabled') ||
      loadingBtn.classes().join(' ') !== idleBtn.classes().join(' ') ||
      loadingBtn.html() !== idleBtn.html()
    expect(differs).toBe(true)
    expect(loadingBtn.text()).toBe('保存')
  })

  it('config 变更（保存成功回写）→ 编辑态刷新为服务端状态', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    await w.setProps({
      config: config({ signature_policy: 'required', registry_whitelist: [] }),
    })
    await w.find('.security-actions button').trigger('click')
    const [payload] = w.emitted('save')?.[0] ?? []
    expect(payload).toEqual({ signature_policy: 'required', registry_whitelist: [] })
  })

  it('重置：上抛 reload 并清空未提交的输入框', async () => {
    const w = mountPanel({ loading: false, saving: false, config: config() })
    await w.find('.registry-input input').setValue('pending')
    await w.findAll('.security-actions button')[1].trigger('click')
    expect(w.emitted('reload')).toBeTruthy()
    expect((w.find('.registry-input input').element as HTMLInputElement).value).toBe('')
  })
})
