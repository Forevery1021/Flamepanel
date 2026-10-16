import { describe, it, expect, vi } from 'vitest'
import { nextTick } from 'vue'
import { mountFp } from '@/components/ui/__tests__/helpers'
import WebsiteDialogs from '@/components/website/WebsiteDialogs.vue'

/**
 * T2-4 第二站拆出的网站弹窗契约（原为 WebsitesView 内联模板）。
 *
 * 编辑态归本组件，因此重点验证**会话生命周期**与**编辑器行为**：
 * 打开即重置 / 编辑以当前行为初值 / 必填校验拦在发请求之前 /
 * 绑定目标随站点类型切换 / 伪静态的可选追加项与规则预览。
 *
 * 注意：FpModal 走 Teleport（appendTo 默认 body），断言必须落在 body 上——
 * `wrapper.html()` 里只有注释占位（T1-1 已记录该陷阱）。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

const messages = {
  website: {
    create: '创建网站',
    edit: '编辑网站',
    name: '名称',
    domain: '域名',
    rootPath: '根目录',
    siteType: '站点类型',
    siteType_static: '静态',
    siteType_reverse_proxy: '反向代理',
    siteType_runtime_php: 'PHP 运行时',
    siteType_container_app: '容器应用',
    nodeId: '节点 ID',
    engine: '引擎',
    sslEnabled: '启用 SSL',
    proxyEnabled: '反向代理开关',
    proxyPass: '代理地址',
    switchEngine: '切换引擎',
    bind: '绑定',
    bindRuntime: '绑定 PHP 运行时',
    bindProject: '绑定项目',
    rewritePreset: '伪静态模板',
    rewriteCurrent: '当前规则',
    rewriteNone: '未套用',
    rewriteHint: '套用说明',
    rewriteClear: '清除套用',
  },
  common: { cancel: '取消', confirm: '确定', required: '此项为必填项' },
}

const site = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'blog',
  domain: 'blog.example.com',
  root_path: '/www/blog',
  node_id: 1,
  status: 'active',
  engine: 'nginx',
  ssl_enabled: false,
  proxy_enabled: false,
  proxy_pass: null,
  site_type: 'static',
  rewrite_preset: null,
  created_at: '',
  ...over,
})

const baseProps = {
  engines: [{ name: 'nginx' }, { name: 'apache' }],
  phpRuntimeOptions: [{ label: 'php 8.2', value: 3 }],
  projectOptions: [{ label: 'my-project', value: 5 }],
  rewritePresetOptions: [
    { label: 'wordpress', value: 'wordpress' },
    { label: 'laravel', value: 'laravel' },
  ],
}

/** 组件方法（openXxx / closeXxx）经 defineExpose 暴露，用例以 vm 直调 */
type DialogsVm = {
  openCreate: () => void
  openEdit: (row: unknown) => void
  openSwitchEngine: (row: unknown) => void
  openBind: (row: unknown) => void
  openRewrite: (row: unknown, status: unknown, options: unknown) => void
  syncRewriteStatus: (status: unknown) => void
  closeCreate: () => void
  closeEdit: () => void
  closeSwitchEngine: () => void
  closeBind: () => void
  closeRewrite: () => void
}

function mountDialogs() {
  document.body.innerHTML = ''
  const w = mountFp(WebsiteDialogs as never, { props: baseProps, messages })
  return { w, vm: w.vm as unknown as DialogsVm, host: document.body }
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

/** 第 index 个可见弹窗 */
function dialog(host: HTMLElement, index = 0): HTMLElement {
  return host.querySelectorAll<HTMLElement>('.p-dialog')[index]
}

function inputs(host: HTMLElement, index = 0): HTMLInputElement[] {
  return Array.from(dialog(host, index).querySelectorAll<HTMLInputElement>('input'))
}

function footerButtons(host: HTMLElement, index = 0): HTMLButtonElement[] {
  return Array.from(dialog(host, index).querySelectorAll('button'))
}

/** 点击弹窗底栏最后一个按钮（主操作） */
async function clickPrimary(host: HTMLElement, index = 0) {
  const btns = footerButtons(host, index)
  btns[btns.length - 1].click()
  await settle()
}

describe('WebsiteDialogs · 新建', () => {
  it('打开即重置表单（每次都是新建会话，不带上次残留）', async () => {
    const { vm, host } = mountDialogs()
    vm.openCreate()
    await settle()
    expect(inputs(host)[0].value).toBe('')

    inputs(host)[0].value = 'ghost'
    await settle()
    vm.closeCreate()
    await settle()
    vm.openCreate()
    await settle()
    expect(inputs(host)[0].value).toBe('')
  })

  it('必填校验：名称/域名缺失时不发事件，并按字段提示', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openCreate()
    await settle()
    await clickPrimary(host)

    expect(w.emitted('create')).toBeUndefined()
    expect(dialog(host).textContent).toContain('此项为必填项')
  })

  it('填写完整后上抛载荷（含反代地址与站点类型）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openCreate()
    await settle()

    const [name, domain, root] = inputs(host)
    name.value = 'new-site'
    name.dispatchEvent(new Event('input'))
    domain.value = 'new.example.com'
    domain.dispatchEvent(new Event('input'))
    root.value = '/www/new'
    root.dispatchEvent(new Event('input'))
    await settle()

    await clickPrimary(host)
    expect(w.emitted('create')?.[0]?.[0]).toEqual(
      expect.objectContaining({
        name: 'new-site',
        domain: 'new.example.com',
        root_path: '/www/new',
        engine: 'nginx',
        site_type: 'static',
        proxy_enabled: false,
      }),
    )
  })
})

describe('WebsiteDialogs · 编辑', () => {
  it('打开即以当前行为初值（编辑不回显是经典隐性缺陷）', async () => {
    const { vm, host } = mountDialogs()
    vm.openEdit(site({ name: 'shop', domain: 'shop.example.com', root_path: '/www/shop' }))
    await settle()

    const [name, domain, root] = inputs(host)
    expect(name.value).toBe('shop')
    expect(domain.value).toBe('shop.example.com')
    expect(root.value).toBe('/www/shop')
  })

  it('缺失字段走与原实现一致的回退（engine 空 → nginx，site_type 空 → static）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openEdit(site({ engine: '', site_type: null, proxy_pass: null }))
    await settle()
    // 回退值落在提交载荷上（DOM 只反映文本字段，枚举/布尔走状态）
    await clickPrimary(host)

    const [, payload] = w.emitted('update')?.[0] as [number, Record<string, unknown>]
    expect(payload.engine).toBe('nginx')
    expect(payload.site_type).toBe('static')
  })

  it('保存上抛 id + 载荷（id 同时进路径与请求体）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openEdit(site({ id: 42 }))
    await settle()
    await clickPrimary(host)

    const [id, payload] = w.emitted('update')?.[0] as [number, Record<string, unknown>]
    expect(id).toBe(42)
    expect(payload.name).toBe('blog')
  })

  it('编辑态清空必填字段：拦在发请求之前', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openEdit(site())
    await settle()

    const name = inputs(host)[0]
    name.value = ''
    name.dispatchEvent(new Event('input'))
    await settle()
    await clickPrimary(host)

    expect(w.emitted('update')).toBeUndefined()
  })
})

describe('WebsiteDialogs · 切换引擎', () => {
  it('打开时以当前行引擎为初值，引擎清单来自 prop', async () => {
    const { vm, host } = mountDialogs()
    vm.openSwitchEngine(site({ engine: 'apache' }))
    await settle()
    expect(dialog(host).textContent).toContain('切换引擎')
    // 下拉选中项以 input 值呈现（FpSelect 底层为 select 组件，选中值在文本上）
    expect(dialog(host).textContent).toContain('apache')
  })

  it('确认后上抛目标 id 与引擎名', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openSwitchEngine(site({ id: 7, engine: 'nginx' }))
    await settle()
    await clickPrimary(host)

    expect(w.emitted('switchEngine')?.[0]).toEqual([7, 'nginx'])
  })
})

describe('WebsiteDialogs · 绑定', () => {
  it('打开以当前行为初值（站点类型 + 已绑定的运行时/项目）', async () => {
    const { vm, host } = mountDialogs()
    vm.openBind(site({ site_type: 'runtime_php', runtime_id: 3 }))
    await settle()

    expect(dialog(host).textContent).toContain('绑定 PHP 运行时')
    expect(dialog(host).textContent).toContain('php 8.2')
  })

  it('static 类型：两个绑定目标下拉都不出现（只留伪静态）', async () => {
    const { vm, host } = mountDialogs()
    vm.openBind(site({ site_type: 'static' }))
    await settle()

    expect(dialog(host).textContent).not.toContain('绑定 PHP 运行时')
    expect(dialog(host).textContent).not.toContain('绑定项目')
    expect(dialog(host).textContent).toContain('伪静态模板')
  })

  it('reverse_proxy 类型：出现项目下拉（而非运行时下拉）', async () => {
    const { vm, host } = mountDialogs()
    vm.openBind(site({ site_type: 'reverse_proxy', project_id: 5 }))
    await settle()

    expect(dialog(host).textContent).toContain('绑定项目')
    expect(dialog(host).textContent).not.toContain('绑定 PHP 运行时')
    expect(dialog(host).textContent).toContain('my-project')
  })

  it('确认后上抛 id + 载荷（未选模板时 rewrite_preset 为 undefined）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openBind(site({ id: 9, site_type: 'runtime_php', runtime_id: 3 }))
    await settle()
    await clickPrimary(host)

    const [id, payload] = w.emitted('bind')?.[0] as [number, Record<string, unknown>]
    expect(id).toBe(9)
    expect(payload).toEqual({
      site_type: 'runtime_php',
      runtime_id: 3,
      project_id: null,
      rewrite_preset: undefined,
    })
  })

  it('清空绑定目标：上抛 null（后端据此解绑，不传 undefined）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openBind(site({ site_type: 'runtime_php', runtime_id: 3 }))
    await settle()

    // 经 FpSelect 的 model 事件清空（与用户点「清除」等价）
    const selects = w.findAllComponents({ name: 'FpSelect' })
    const runtimeSelect = selects.find((s) =>
      String((s.props() as { label?: string }).label ?? '').includes('绑定 PHP 运行时'),
    )
    expect(runtimeSelect).toBeTruthy()
    runtimeSelect!.vm.$emit('update:modelValue', undefined)
    await settle()
    await clickPrimary(host)

    const [, payload] = w.emitted('bind')?.[0] as [number, Record<string, unknown>]
    expect(payload.runtime_id).toBeNull()
  })
})

describe('WebsiteDialogs · 伪静态', () => {
  const status = { preset: 'wordpress', rules: 'rewrite ^/old$ /new last;', supported: [] }

  it('打开以服务端状态为初值：预览展示真实规则文本', async () => {
    const { vm, host } = mountDialogs()
    vm.openRewrite(site(), status, [{ label: 'wordpress', value: 'wordpress' }])
    await settle()

    expect(dialog(host).textContent).toContain('rewrite ^/old$ /new last;')
    expect(dialog(host).textContent).toContain('当前规则')
  })

  it('已套用时出现「清除套用」按钮；确认套用上抛 id + 模板名', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openRewrite(site({ id: 4 }), status, [{ label: 'wordpress', value: 'wordpress' }])
    await settle()

    const labels = footerButtons(host).map((b) => b.textContent)
    expect(labels).toContain('清除套用')

    await clickPrimary(host)
    expect(w.emitted('applyRewrite')?.[0]).toEqual([4, 'wordpress'])
  })

  it('未套用：无「清除套用」按钮，且确认按钮禁用（不能提交空模板）', async () => {
    const { vm, host } = mountDialogs()
    vm.openRewrite(site({ rewrite_preset: null }), { preset: null, rules: '', supported: [] }, [])
    await settle()

    const labels = footerButtons(host).map((b) => b.textContent)
    expect(labels).not.toContain('清除套用')
    expect(footerButtons(host).at(-1)?.disabled).toBe(true)
  })

  it('清除上抛的是最后一次打开的站点 id（会话不串行错位）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openRewrite(site({ id: 4 }), status, [])
    await settle()
    vm.closeRewrite()
    await settle()
    vm.openRewrite(site({ id: 8 }), status, [])
    await settle()

    const clear = footerButtons(host).find((b) => b.textContent === '清除套用')!
    clear.click()
    await settle()
    expect(w.emitted('clearRewrite')?.[0]).toEqual([8])
  })

  it('写操作成功后 syncRewriteStatus 回写预览（rules 用服务端最新文本）', async () => {
    const { vm, host } = mountDialogs()
    vm.openRewrite(site(), status, [])
    await settle()

    vm.syncRewriteStatus({ preset: 'laravel', rules: 'server-generated', supported: [] })
    await settle()
    expect(dialog(host).textContent).toContain('server-generated')
  })
})

describe('WebsiteDialogs · 会话隔离', () => {
  it('关闭再打开编辑另一行：不残留上一行的值', async () => {
    const { vm, host } = mountDialogs()
    vm.openEdit(site({ name: 'first', domain: 'first.com' }))
    await settle()
    vm.closeEdit()
    await settle()
    vm.openEdit(site({ name: 'second', domain: 'second.com' }))
    await settle()

    expect(inputs(host)[0].value).toBe('second')
    expect(inputs(host)[1].value).toBe('second.com')
  })
})
