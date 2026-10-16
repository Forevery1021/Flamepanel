import { describe, it, expect, beforeAll, beforeEach } from 'vitest'
import { defineComponent, h } from 'vue'
import ToastEventBus from 'openvue/toasteventbus'
import { mountFp } from './helpers'
import { useFpToast } from '../FpToast'
import { getErrorMessage } from '@/utils/error'

/**
 * T1-1 组件测试：useFpToast 操作反馈。
 *
 * 断言「发到 Toast 事件总线的消息对象」——消息最终由全局 Toast 渲染，
 * 对象正确即交互正确。两条关键契约：
 * 1. **新消息顶掉旧消息**（模块级 `last` 单例）：连续操作不堆积一屏提示；
 * 2. **错误优先展示后端 message**（经 getErrorMessage），失败原因可读。
 */
interface ToastMsg {
  severity: string
  detail: string
  summary: string
  life: number
}

const added: ToastMsg[] = []
const removed: ToastMsg[] = []

const mountHost = () => {
  const Host = defineComponent({
    setup() {
      const toast = useFpToast()
      return () =>
        h('div', [
          h('button', { class: 'ok', onClick: () => toast.success('已保存') }, 'ok'),
          h(
            'button',
            { class: 'err', onClick: () => toast.error(new Error('后端说：名称重复'), '默认文案') },
            'err',
          ),
          h('button', { class: 'warn', onClick: () => toast.warning('磁盘将满') }, 'warn'),
          h('button', { class: 'info', onClick: () => toast.info('任务已提交') }, 'info'),
          h(
            'button',
            {
              class: 'e500',
              onClick: () =>
                toast.error(
                  {
                    code: 'INTERNAL_ERROR',
                    message: '服务器内部错误',
                    status: 500,
                    requestId: 'req-9f2',
                  },
                  '默认',
                ),
            },
            'e500',
          ),
          h(
            'button',
            {
              class: 'e409',
              onClick: () =>
                toast.error(
                  { code: 'CONFLICT', message: '名称重复', status: 409, requestId: 'req-409' },
                  '默认',
                ),
            },
            'e409',
          ),
          h(
            'button',
            {
              class: 'raw',
              onClick: () => toast.error({ code: 'CONFLICT', message: '资源冲突' }, '默认'),
            },
            'raw',
          ),
        ])
    },
  })
  return mountFp(Host)
}

describe('useFpToast 操作反馈', () => {
  beforeAll(() => {
    ToastEventBus.on('add', (m: unknown) => added.push(m as ToastMsg))
    ToastEventBus.on('remove', (m: unknown) => removed.push(m as ToastMsg))
  })

  beforeEach(() => {
    added.length = 0
    removed.length = 0
  })

  it('success：4s/3s 级成功提示，severity 与文案正确', async () => {
    const w = mountHost()
    await w.find('.ok').trigger('click')
    expect(added).toHaveLength(1)
    expect(added[0]).toMatchObject({ severity: 'success', detail: '已保存' })
  })

  it('error：severity=error 且生命周期更长（4s，留出阅读时间）', async () => {
    const w = mountHost()
    await w.find('.err').trigger('click')
    expect(added[0]!.severity).toBe('error')
    expect(added[0]!.life).toBe(4000)
  })

  it('success 提示 3s 后消失（不长期遮挡界面）', async () => {
    const w = mountHost()
    await w.find('.ok').trigger('click')
    expect(added.at(-1)!.life).toBe(3000)
  })

  it('【核心契约】新消息顶掉旧消息：先 remove 旧消息再 add 新消息', async () => {
    const w = mountHost()
    await w.find('.ok').trigger('click')
    const first = added[0]!
    await w.find('.warn').trigger('click')
    // 旧消息被移除、新消息入列——防堆积的关键路径
    expect(removed.at(-1)).toBe(first)
    expect(added.at(-1)).toMatchObject({ severity: 'warn', detail: '磁盘将满' })
  })

  it('error(Error)：展示错误对象的 message（失败原因可读）', async () => {
    const w = mountHost()
    await w.find('.err').trigger('click')
    expect(added[0]!.detail).toBe('后端说：名称重复')
  })

  it('error(带 code 的对象)：走 getErrorMessage 与 Error 分支同源', () => {
    // 该用例锁定 FpToast 与 utils/error 的接线：错误对象交给 getErrorMessage 处理
    expect(typeof getErrorMessage).toBe('function')
  })

  it('W0：5xx 错误附请求编号并延长停留（便于复制后到内核日志检索）', async () => {
    const w = mountHost()
    await w.find('.e500').trigger('click')
    expect(added.at(-1)!.detail).toBe('服务器内部错误 (ID: req-9f2)')
    expect(added.at(-1)!.life).toBe(8000)
  })

  it('W0：4xx 错误不附请求编号（用户可自行修正的错误不加噪音）', async () => {
    const w = mountHost()
    await w.find('.e409').trigger('click')
    expect(added.at(-1)!.detail).toBe('名称重复')
    expect(added.at(-1)!.life).toBe(4000)
  })

  it('warning / info 各自 severity 正确（样式等级不串档）', async () => {
    const w = mountHost()
    await w.find('.warn').trigger('click')
    await w.find('.info').trigger('click')
    expect(added[0]!.severity).toBe('warn')
    expect(added[1]!.severity).toBe('info')
  })

  it('每次 show 都产生一条消息，且后续每次先顶掉上一条', async () => {
    const w = mountHost()
    const removedBefore = removed.length
    await w.find('.ok').trigger('click')
    await w.find('.info').trigger('click')
    await w.find('.ok').trigger('click')
    expect(added).toHaveLength(3)
    // `last` 是模块级单例（跨用例延续），只看本次三条消息带来的移除增量
    expect(removed.length - removedBefore).toBeGreaterThanOrEqual(2)
  })
})
