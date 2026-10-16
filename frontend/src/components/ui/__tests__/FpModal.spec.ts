import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mountFp, createHost } from './helpers'
import FpModal from '../FpModal.vue'

/**
 * T1-1 组件测试：FpModal 显隐契约与插槽装配。
 *
 * Dialog 走 Teleport（appendTo），断言必须落在挂载宿主的 DOM 上而非
 * wrapper.html()——wrapper 里只有一行注释占位，这是浮层组件测试最常见的
 * 「断言永远为空」陷阱。此处通过 `appendTo` 把弹层收敛到测试自建宿主，
 * 避免污染 document.body 影响其他用例。
 */
const mountModal = (props: Record<string, unknown> = {}, slots: Record<string, string> = {}) => {
  const host = createHost()
  const w = mountFp(FpModal, {
    props: { modelValue: true, header: '新建用户', appendTo: host, ...props },
    slots,
    attachTo: host,
  })
  return { w, host }
}

describe('FpModal 弹窗', () => {
  it('modelValue=true：渲染标题与主体内容', async () => {
    const { host } = mountModal({}, { default: '<div class="body">表单主体</div>' })
    await nextTick()
    await nextTick()
    expect(host.querySelector('.p-dialog-title')?.textContent).toBe('新建用户')
    expect(host.querySelector('.body')?.textContent).toBe('表单主体')
  })

  it('modelValue=false：不渲染对话框（v-model 控制显隐）', async () => {
    const { host } = mountModal({ modelValue: false })
    await nextTick()
    await nextTick()
    expect(host.querySelector('.p-dialog')).toBeNull()
  })

  it('footer 插槽存在时渲染底栏容器；缺省则不渲染', async () => {
    const withFooter = mountModal({}, { footer: '<button class="ok">确定</button>' })
    await nextTick()
    await nextTick()
    expect(withFooter.host.querySelector('.fp-modal__footer')).not.toBeNull()
    // 业务侧按钮经插槽原样进入底栏
    expect(withFooter.host.querySelector('.fp-modal__footer .ok')?.textContent).toBe('确定')

    const noFooter = mountModal()
    await nextTick()
    await nextTick()
    expect(noFooter.host.querySelector('.fp-modal__footer')).toBeNull()
  })

  it('点击关闭按钮发出 update:modelValue=false（父组件据此收起弹窗）', async () => {
    const { w, host } = mountModal()
    await nextTick()
    await nextTick()
    const close = host.querySelector('.p-dialog-header button') as HTMLButtonElement
    expect(close).not.toBeNull()
    close.click()
    await nextTick()
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })

  it('closable=false：不渲染关闭按钮（强制走底栏操作）', async () => {
    const { host } = mountModal({ closable: false })
    await nextTick()
    await nextTick()
    expect(host.querySelector('.p-dialog-header button')).toBeNull()
  })

  it('显示时带 fp-modal 类名（主题令牌入口）', async () => {
    const { host } = mountModal()
    await nextTick()
    await nextTick()
    expect(host.querySelector('.p-dialog')?.classList.contains('fp-modal')).toBe(true)
  })

  it('size 档位：默认 md，映射为 fp-modal--{size} 类（宽度由类而非内联 style 决定）', async () => {
    const md = mountModal()
    await nextTick()
    await nextTick()
    expect(md.host.querySelector('.p-dialog')?.classList.contains('fp-modal--md')).toBe(true)
    const lg = mountModal({ size: 'lg' })
    await nextTick()
    await nextTick()
    expect(lg.host.querySelector('.p-dialog')?.classList.contains('fp-modal--lg')).toBe(true)
  })

  it('【无障碍】对话框语义：role=dialog + aria-modal，标题经 aria-labelledby 命名', async () => {
    const { host } = mountModal({}, { default: '<input class="first" />' })
    await nextTick()
    await nextTick()
    const dialog = host.querySelector('.p-dialog') as HTMLElement
    expect(dialog.getAttribute('role')).toBe('dialog')
    expect(dialog.getAttribute('aria-modal')).toBe('true')
    const labelledBy = dialog.getAttribute('aria-labelledby')
    expect(labelledBy).toBeTruthy()
    expect(host.querySelector(`#${labelledBy}`)?.textContent).toBe('新建用户')
  })
})
