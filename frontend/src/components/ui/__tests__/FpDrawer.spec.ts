import { describe, it, expect, afterEach } from 'vitest'
import { nextTick } from 'vue'
import type { VueWrapper } from '@vue/test-utils'
import { mountFp } from './helpers'
import FpDrawer from '../FpDrawer.vue'

/**
 * T1-1 组件测试：FpDrawer 侧边抽屉。
 *
 * 浮层装配差异（与 FpModal 不同）：Drawer 固定 Teleport 到 `document.body`，
 * 不提供 `appendTo`（其 `inheritAttrs: false` 会吞掉该属性）——断言只能落在
 * body 上，故每个用例后必须 unmount 并清理残留节点，否则会污染后续用例。
 *
 * 业务关注点是 `position` 决定贴边方向：详情/编辑抽屉默认右侧。
 */
let wrapper: VueWrapper | null = null

function mountDrawer(props: Record<string, unknown> = {}, slots: Record<string, string> = {}) {
  wrapper = mountFp(FpDrawer, {
    props: { modelValue: true, header: '容器详情', ...props },
    slots,
  })
  return wrapper
}

/** 等 Teleport + Transition 完成（抽屉有 appear 过渡，单次 tick 不够） */
async function settle() {
  for (let i = 0; i < 4; i++) await nextTick()
}

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  document.body.innerHTML = ''
})

describe('FpDrawer 抽屉', () => {
  it('modelValue=true：渲染标题与主体内容', async () => {
    mountDrawer({}, { default: '<div class="body">详情内容</div>' })
    await settle()
    expect(document.body.querySelector('.p-drawer-title')?.textContent).toBe('容器详情')
    expect(document.body.querySelector('.body')?.textContent).toBe('详情内容')
  })

  it('modelValue=false：不渲染抽屉', async () => {
    mountDrawer({ modelValue: false })
    await settle()
    expect(document.body.querySelector('.p-drawer')).toBeNull()
  })

  it('默认位置右侧（position=right）', async () => {
    mountDrawer()
    await settle()
    const root = document.body.querySelector('.p-drawer')
    expect(root).not.toBeNull()
    const marker = document.body.querySelector('.p-drawer-right')
    expect(marker).not.toBeNull()
  })

  it('position=left：渲染为左侧抽屉', async () => {
    mountDrawer({ position: 'left' })
    await settle()
    expect(document.body.querySelector('.p-drawer-left')).not.toBeNull()
  })

  it('footer 插槽：提供时渲染底栏容器与内容', async () => {
    mountDrawer({}, { footer: '<button class="ok">保存</button>' })
    await settle()
    expect(document.body.querySelector('.fp-drawer__footer')).not.toBeNull()
    expect(document.body.querySelector('.fp-drawer__footer .ok')?.textContent).toBe('保存')
  })

  it('无 footer 插槽：不渲染底栏（保持内容区通高）', async () => {
    mountDrawer()
    await settle()
    expect(document.body.querySelector('.fp-drawer__footer')).toBeNull()
  })

  it('关闭按钮发出 update:modelValue=false', async () => {
    const w = mountDrawer()
    await settle()
    const close = document.body.querySelector('.p-drawer-close-button') as HTMLButtonElement
    expect(close).not.toBeNull()
    close.click()
    await nextTick()
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })
})
