import { describe, it, expect, vi } from 'vitest'
import { nextTick } from 'vue'
import { mountFp } from '@/components/ui/__tests__/helpers'
import CertificateDialogs from '@/components/website/CertificateDialogs.vue'

/**
 * T2-4 第二站拆出的证书弹窗契约（原为 WebsitesView 内联模板）。
 *
 * 重点验证四条**载荷语义**（它们决定证书签发的正确性，此前埋在视图模板里）：
 * ① 主域名必填，备用域名按逗号拆分并**过滤空项**（不产生空串 SAN）；
 * ② 上传弹窗三个字段全必填（私钥缺失必然被后端拒绝，前端先拦）；
 * ③ DNS-01 弹窗的厂商账号下拉**只列启用项**；
 * ④ 绑定弹窗验证「记录未绑定 / 已绑定」两种初值。
 *
 * 注意：FpModal 走 Teleport（appendTo 默认 body），断言必须落在 body 上。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

const messages = {
  certificate: {
    issueAcme: 'ACME 签发',
    issueSelfSigned: '自签证书',
    upload: '上传证书',
    bind: '绑定网站',
    primaryDomain: '主域名',
    altDomains: '备用域名（逗号分隔）',
    domains: '域名列表',
    certPem: '证书 PEM（全链）',
    keyPem: '私钥 PEM',
    dns01: 'DNS-01 验证',
    dns01Hint: '泛域名必须使用 DNS-01',
    dnsAccount: 'DNS 厂商账号',
    dnsAccountPlaceholder: '选择厂商账号',
    webroot: '网站根目录（HTTP-01）',
    boundWebsite: '关联网站',
    issue: '签发',
  },
  common: { cancel: '取消', confirm: '确定' },
}

type DialogsVm = {
  openAcme: () => void
  openSelf: () => void
  openUpload: () => void
  openBind: (row: unknown) => void
  closeAcme: () => void
  closeSelf: () => void
  closeUpload: () => void
  closeBind: () => void
}

function mountDialogs(props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  const w = mountFp(CertificateDialogs as never, { props, messages })
  return { w, vm: w.vm as unknown as DialogsVm, host: document.body }
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

function dialog(host: HTMLElement, index = 0): HTMLElement {
  return host.querySelectorAll<HTMLElement>('.p-dialog')[index]
}

function inputs(host: HTMLElement, index = 0): HTMLInputElement[] {
  return Array.from(dialog(host, index).querySelectorAll<HTMLInputElement>('input'))
}

function textareas(host: HTMLElement, index = 0): HTMLTextAreaElement[] {
  return Array.from(dialog(host, index).querySelectorAll<HTMLTextAreaElement>('textarea'))
}

function footerButtons(host: HTMLElement, index = 0): HTMLButtonElement[] {
  return Array.from(dialog(host, index).querySelectorAll('button'))
}

async function clickPrimary(host: HTMLElement, index = 0) {
  const btns = footerButtons(host, index)
  btns[btns.length - 1].click()
  await settle()
}

/** 写入受控输入并派发事件（Vue 的 v-model 依赖 input 事件） */
async function type(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  el.value = value
  el.dispatchEvent(new Event('input'))
  await settle()
}

describe('CertificateDialogs · ACME 签发', () => {
  it('主域名必填：为空时上抛 required 且不发签发事件', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openAcme()
    await settle()
    await clickPrimary(host)

    expect(w.emitted('required')).toBeTruthy()
    expect(w.emitted('acme')).toBeUndefined()
  })

  it('备用域名按逗号拆分并过滤空项（"a, ,b" 不产生空串 SAN）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openAcme()
    await settle()
    const [primary, alt] = inputs(host)
    await type(primary, 'example.com')
    await type(alt, ' www.example.com , , api.example.com ')
    await clickPrimary(host)

    expect(w.emitted('acme')?.[0]?.[0]).toEqual(
      expect.objectContaining({
        domains: ['example.com', 'www.example.com', 'api.example.com'],
        dns01: false,
      }),
    )
  })

  it('HTTP-01 模式：展示 webroot 输入框，不展示厂商账号下拉', async () => {
    const { w, vm, host } = mountDialogs({
      dnsAccounts: [{ id: 1, name: 'cf', provider_type: 'cloudflare', enabled: true }],
    })
    vm.openAcme()
    await settle()

    expect(dialog(host).textContent).toContain('网站根目录（HTTP-01）')
    expect(dialog(host).textContent).not.toContain('DNS 厂商账号')
    expect(w.findAllComponents({ name: 'FpSelect' })).toHaveLength(0)
  })

  it('切换到 DNS-01：出现厂商账号下拉与提示，webroot 输入框消失', async () => {
    const { w, vm, host } = mountDialogs({
      dnsAccounts: [{ id: 1, name: 'cf', provider_type: 'cloudflare', enabled: true }],
    })
    vm.openAcme()
    await settle()

    // FpSwitch 的 model 事件即用户拨动开关
    w.findComponent({ name: 'FpSwitch' }).vm.$emit('update:modelValue', true)
    await settle()

    expect(dialog(host).textContent).toContain('DNS 厂商账号')
    expect(dialog(host).textContent).not.toContain('网站根目录（HTTP-01）')
    // 提示条：FpInlineMessage 经 $attrs 透传（OpenVue InlineMessage 无 text 插槽），
    // 故提示文案落在 DOM 属性上而非文本节点——断言属性以锁「提示确实下发」
    const hint = dialog(host).querySelector('[text]')
    expect(hint?.getAttribute('text')).toContain('泛域名必须使用 DNS-01')
  })

  it('厂商下拉只列启用账号（禁用账号不得成为可选目标）', async () => {
    const { w, vm } = mountDialogs({
      dnsAccounts: [
        { id: 1, name: 'cf', provider_type: 'cloudflare', enabled: true },
        { id: 2, name: 'ali', provider_type: 'aliyun', enabled: false },
      ],
    })
    vm.openAcme()
    await settle()
    w.findComponent({ name: 'FpSwitch' }).vm.$emit('update:modelValue', true)
    await settle()

    // FpSelect 未声明 `options` prop（经 $attrs 透传给底层 Select），故从 $attrs 读取
    const select = w
      .findAllComponents({ name: 'FpSelect' })
      .find((s) => String((s.props() as { label?: string }).label ?? '').includes('DNS 厂商账号'))!
    const options = (select.vm.$attrs as { options: { label: string }[] }).options
    expect(options.map((o) => o.label)).toEqual(['cf (cloudflare)'])
  })

  it('重新打开即重置（DNS-01 开关不残留上一轮的勾选）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openAcme()
    await settle()
    w.findComponent({ name: 'FpSwitch' }).vm.$emit('update:modelValue', true)
    await settle()
    vm.closeAcme()
    await settle()
    vm.openAcme()
    await settle()

    expect(dialog(host).textContent).toContain('网站根目录（HTTP-01）')
  })
})

describe('CertificateDialogs · 自签', () => {
  it('主域名必填：为空时上抛 required', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openSelf()
    await settle()
    await clickPrimary(host)

    expect(w.emitted('required')).toBeTruthy()
    expect(w.emitted('selfSigned')).toBeUndefined()
  })

  it('上抛主域名 + 备用域名（解析后的数组）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openSelf()
    await settle()
    const [primary, alt] = inputs(host)
    await type(primary, 'intranet.local')
    await type(alt, 'a.intranet.local,b.intranet.local')
    await clickPrimary(host)

    expect(w.emitted('selfSigned')?.[0]?.[0]).toEqual([
      'intranet.local',
      'a.intranet.local',
      'b.intranet.local',
    ])
  })
})

describe('CertificateDialogs · 上传', () => {
  it('三个字段全必填：缺私钥时拦在发请求之前', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openUpload()
    await settle()
    const [domains] = inputs(host)
    await type(domains, 'uploaded.com')
    await clickPrimary(host)

    expect(w.emitted('required')).toBeTruthy()
    expect(w.emitted('upload')).toBeUndefined()
  })

  it('齐全后上抛载荷（PEM 原样透传，不裁剪首尾空白）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openUpload()
    await settle()
    const [domains] = inputs(host)
    const [certPem, keyPem] = textareas(host)
    await type(domains, 'uploaded.com')
    await type(certPem, '\n-----BEGIN CERTIFICATE-----\n')
    await type(keyPem, '-----BEGIN PRIVATE KEY-----\n')
    await clickPrimary(host)

    expect(w.emitted('upload')?.[0]?.[0]).toEqual({
      domains: ['uploaded.com'],
      cert_pem: '\n-----BEGIN CERTIFICATE-----\n',
      key_pem: '-----BEGIN PRIVATE KEY-----\n',
    })
  })

  it('域名列多值：解析为多个 SAN（主域名首项兜底逻辑不误伤正常输入）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openUpload()
    await settle()
    const [domains] = inputs(host)
    const [certPem, keyPem] = textareas(host)
    await type(domains, 'a.com, b.com')
    await type(certPem, 'C')
    await type(keyPem, 'K')
    await clickPrimary(host)

    expect((w.emitted('upload')?.[0]?.[0] as { domains: string[] }).domains).toEqual([
      'a.com',
      'b.com',
    ])
  })
})

describe('CertificateDialogs · 绑定网站', () => {
  it('未绑定记录：输入框初值为空', async () => {
    const { vm, host } = mountDialogs()
    vm.openBind({ id: 1, website_id: null })
    await settle()

    expect(dialog(host).textContent).toContain('关联网站')
    expect(inputs(host)[0].value).toBe('')
  })

  it('已绑定记录：以当前 website_id 为初值（编辑不回显）', async () => {
    const { vm, host } = mountDialogs()
    vm.openBind({ id: 1, website_id: 7 })
    await settle()

    expect(inputs(host)[0].value).toBe('7')
  })

  it('清空输入 → 上抛 null（后端据此解绑）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openBind({ id: 3, website_id: 7 })
    await settle()
    await type(inputs(host)[0], '')

    // FpNumber 的空值经 model 事件上报 null；用户清空后确认
    w.findComponent({ name: 'FpNumber' }).vm.$emit('update:modelValue', null)
    await settle()
    await clickPrimary(host)

    expect(w.emitted('bind')?.[0]).toEqual([3, null])
  })

  it('上抛目标 id 与输入的网站 id（会话不串行错位）', async () => {
    const { w, vm, host } = mountDialogs()
    vm.openBind({ id: 3, website_id: null })
    await settle()
    w.findComponent({ name: 'FpNumber' }).vm.$emit('update:modelValue', 12)
    await settle()
    await clickPrimary(host)

    expect(w.emitted('bind')?.[0]).toEqual([3, 12])
  })
})
