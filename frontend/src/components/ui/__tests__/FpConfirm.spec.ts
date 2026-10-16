import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest'
import { defineComponent, h } from 'vue'
import ConfirmationEventBus from 'openvue/confirmationeventbus'
import { mountFp } from './helpers'
import { useFpConfirm } from '../FpConfirm'

/**
 * T1-1 组件测试：useFpConfirm 确认弹窗。
 *
 * 破坏性操作（删除站点 / 卸载应用）统一走确认弹窗。此处断言「传给底层的
 * 选项对象」而非渲染结果——弹窗由全局 ConfirmationService 渲染，选项正确
 * 即等价于交互正确，且不受浮层宿主的干扰。
 *
 * 覆盖两类回归风险：
 * - 危险操作用红色确认按钮（默认语义），仅显式 `danger: false` 才降级；
 * - accept/reject 回调原样透传（按钮点下去的副作用由调用方决定）。
 */
interface ConfirmOption {
  message: string
  header: string
  icon: string
  acceptLabel: string
  rejectLabel: string
  acceptClass: string
  accept?: () => void
}

/**
 * 事件总线是模块级单例，监听器跨用例累积。这里只在 beforeAll 注册**一次**，
 * 每个用例前清空累积数组——避免「用几条用例就收到几条重复事件」的假计数。
 */
const captured: ConfirmOption[] = []

const mountHost = (options: Parameters<ReturnType<typeof useFpConfirm>['confirmAction']>[0]) => {
  const Host = defineComponent({
    setup() {
      const { confirmAction } = useFpConfirm()
      return () => h('button', { class: 'trigger', onClick: () => confirmAction(options) }, 'go')
    },
  })
  return mountFp(Host)
}

describe('useFpConfirm 确认弹窗', () => {
  beforeAll(() => {
    ConfirmationEventBus.on('confirm', (o: unknown) => captured.push(o as ConfirmOption))
  })

  beforeEach(() => {
    captured.length = 0
  })

  it('默认选项：标题/图标/中英文按钮文案齐备', async () => {
    const w = mountHost({ message: '确定删除该站点？' })
    await w.find('.trigger').trigger('click')
    expect(captured).toHaveLength(1)
    const opt = captured[0]!
    expect(opt.message).toBe('确定删除该站点？')
    expect(opt.header).toBe('确认操作')
    expect(opt.icon).toBe('oi oi-exclamation-triangle')
    expect(opt.acceptLabel).toBe('确定')
    expect(opt.rejectLabel).toBe('取消')
  })

  it('默认按危险操作渲染（红色确认按钮）', async () => {
    const w = mountHost({ message: 'x' })
    await w.find('.trigger').trigger('click')
    expect(captured[0]!.acceptClass).toBe('p-button-danger')
  })

  it('danger=false：显式降级为主色按钮（非破坏性确认）', async () => {
    const w = mountHost({ message: 'x', danger: false })
    await w.find('.trigger').trigger('click')
    expect(captured[0]!.acceptClass).toBe('p-button-primary')
  })

  it('accept / reject 回调原样透传', async () => {
    const accept = vi.fn()
    const reject = vi.fn()
    const w = mountHost({ message: 'x', accept, reject })
    await w.find('.trigger').trigger('click')
    captured[0]!.accept?.()
    expect(accept).toHaveBeenCalledTimes(1)
    // 底层仅在用户确认时调用 accept，reject 未触发
    expect(reject).not.toHaveBeenCalled()
  })

  it('自定义标题与文案覆盖默认值', async () => {
    const w = mountHost({
      message: 'x',
      header: '删除备份',
      acceptLabel: '删除',
      rejectLabel: '保留',
    })
    await w.find('.trigger').trigger('click')
    const opt = captured[0]!
    expect(opt.header).toBe('删除备份')
    expect(opt.acceptLabel).toBe('删除')
    expect(opt.rejectLabel).toBe('保留')
  })
})
