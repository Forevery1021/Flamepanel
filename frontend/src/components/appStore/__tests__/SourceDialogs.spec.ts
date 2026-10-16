import { describe, it, expect, vi } from 'vitest'
import { nextTick } from 'vue'
import { mountFp } from '@/components/ui/__tests__/helpers'
import SourceDialogs from '@/components/appStore/SourceDialogs.vue'

/**
 * T2-4 拆分后的远程源弹窗契约（原为 AppStoreView 内联模板）。
 *
 * 编辑态归本组件（`useAppStoreSources` 只保留请求与确认分支），故重点验证
 * **会话生命周期**：打开即重置、打开公钥弹窗以当前行为初值、
 * 提交上抛的载荷就是当前输入。
 *
 * 注意：FpModal 走 Teleport（appendTo 默认 body），断言必须落在自建宿主的
 * DOM 上——wrapper.html() 里只有注释占位（T1-1 已记录该陷阱）。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

const messages = {
  appStore: {
    addSource: '添加远程源',
    colSourceName: '名称',
    colSourceType: '类型',
    colSourceUrl: '地址',
    sourcePublicKey: '公钥',
    sourcePublicKeyPlaceholder: '留空则不验签',
    addAndSync: '添加',
    editPublicKey: '编辑公钥',
    editPublicKeyHint: '清空即卸载信任锚',
  },
  common: { cancel: '取消', save: '保存' },
}

/**
 * 弹窗经 FpModal 走 Teleport 到 document.body（组件未透传 appendTo），
 * 故断言宿主统一取 body；每个用例前清理残留弹层，避免用例间互相污染。
 */
function mountDialogs(props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  const w = mountFp(SourceDialogs as never, { props, messages })
  return { w, host: document.body }
}

/** 弹窗（可能在 host 的 teleport 容器内）里的输入框 */
function inputs(host: HTMLElement): HTMLInputElement[] {
  return Array.from(host.querySelectorAll<HTMLInputElement>('.p-dialog input'))
}

/** 某个弹窗（按序）底栏按钮 */
function footerButtons(host: HTMLElement, dialogIndex = 0): HTMLButtonElement[] {
  const dialogs = host.querySelectorAll('.p-dialog')
  return Array.from(dialogs[dialogIndex]?.querySelectorAll('button') ?? [])
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

describe('SourceDialogs · 添加源', () => {
  it('打开即重置表单（每次都是新建会话，不带上次残留）', async () => {
    const { w, host } = mountDialogs({ modelValue: true })
    await settle()
    expect(inputs(host)[0].value).toBe('')

    // 输入 → 关闭 → 重开：输入必须清空
    inputs(host)[0].value = 'ghost'
    await w.setProps({ modelValue: false })
    await settle()
    await w.setProps({ modelValue: true })
    await settle()
    expect(inputs(host)[0].value).toBe('')
  })

  it('URL 占位符随源类型切换（git / http 各给真实示例）', async () => {
    const { w, host } = mountDialogs({ modelValue: true })
    await settle()
    const urlPlaceholder = () =>
      Array.from(host.querySelectorAll<HTMLInputElement>('.p-dialog input'))[1].placeholder

    expect(urlPlaceholder()).toContain('github.com/1Panel-dev/appstore')

    // 通过 FpSelect（第二个下拉）切换类型
    const selects = (w.vm as unknown as { $: unknown }) && w.findAllComponents({ name: 'FpSelect' })
    selects[0].vm.$emit('update:modelValue', 'http')
    await settle()
    expect(urlPlaceholder()).toContain('apps.example.com/manifest.json')
  })

  it('提交上抛当前输入载荷（含公钥原值）', async () => {
    const { w, host } = mountDialogs({ modelValue: true })
    await settle()
    const [name, url, key] = inputs(host)
    name.value = 'my-source'
    name.dispatchEvent(new Event('input'))
    url.value = 'https://git.example.com/store'
    url.dispatchEvent(new Event('input'))
    key.value = 'pubkey'
    key.dispatchEvent(new Event('input'))
    await settle()

    const btns = footerButtons(host, 0)
    btns[btns.length - 1].click()
    await settle()

    expect(w.emitted('create')?.[0]?.[0]).toEqual({
      name: 'my-source',
      source_type: 'git',
      url: 'https://git.example.com/store',
      public_key: 'pubkey',
    })
  })

  it('创建中按钮进入 loading 态（防重复提交）', async () => {
    const idle = mountDialogs({ modelValue: true, creating: false })
    await settle()
    const idleHtml = footerButtons(idle.host, 0).at(-1)?.outerHTML
    const idleText = footerButtons(idle.host, 0).at(-1)?.textContent

    const busy = mountDialogs({ modelValue: true, creating: true })
    await settle()
    const busyHtml = footerButtons(busy.host, 0).at(-1)?.outerHTML

    expect(idleText).toBe('添加')
    expect(busyHtml).not.toBe(idleHtml)
  })
})

describe('SourceDialogs · 公钥编辑', () => {
  const source = {
    id: 1,
    name: 'official',
    public_key: 'OLD',
    source_type: 'git',
    url: '',
    enabled: true,
    created_at: '',
    updated_at: '',
  }

  it('打开时以当前行公钥为初值（不是空表单）', async () => {
    const { host } = mountDialogs({ modelValue: false, keyVisible: true, source })
    await settle()
    // 两个弹窗都打开与否取决于 keyVisible；此处仅公钥弹窗可见
    const dialogs = host.querySelectorAll('.p-dialog')
    expect(dialogs.length).toBeGreaterThanOrEqual(1)
    const input = dialogs[0].querySelector<HTMLInputElement>('input')
    expect(input?.value).toBe('OLD')
  })

  it('历史未签名源（public_key=null）→ 初值为空串', async () => {
    const { host } = mountDialogs({ modelValue: false, keyVisible: true, source: { ...source, public_key: null } })
    await settle()
    const input = host.querySelector<HTMLInputElement>('.p-dialog input')
    expect(input?.value).toBe('')
  })

  it('标题带源名（编辑对象明确）', async () => {
    const { host } = mountDialogs({ modelValue: false, keyVisible: true, source })
    await settle()
    expect(host.textContent).toContain('official')
  })

  it('提示文案说明「清空即卸载信任锚」语义', async () => {
    const { host } = mountDialogs({ modelValue: false, keyVisible: true, source })
    await settle()
    expect(host.textContent).toContain('清空即卸载信任锚')
  })

  it('保存上抛当前输入值（确认分支在 useAppStoreSources）', async () => {
    const { w, host } = mountDialogs({ modelValue: false, keyVisible: true, source })
    await settle()
    const input = host.querySelector<HTMLInputElement>('.p-dialog input') as HTMLInputElement
    input.value = 'NEW'
    input.dispatchEvent(new Event('input'))
    await settle()

    const btns = footerButtons(host, 0)
    btns[btns.length - 1].click()
    await settle()
    expect(w.emitted('saveKey')?.[0]?.[0]).toBe('NEW')
  })
})
