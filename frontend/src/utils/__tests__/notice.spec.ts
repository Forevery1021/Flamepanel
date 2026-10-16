import { describe, it, expect, beforeEach } from 'vitest'
import { emitNotice, onNotice, resetNoticeChannel, type NoticeMessage } from '@/utils/notice'

/**
 * T0-1：非组件上下文通知通道（路由守卫 → UI）。
 *
 * 守卫在组件树之外运行，`useFpToast()` 的 `inject` 会失败；且刷新页面时守卫先于
 * 布局挂载执行——若直接丢弃消息，「无权限」提示在硬刷场景静默消失（用户只看到
 * 自己被弹回仪表盘，不知原因）。故通道必须**先暂存、后补投**。
 */
describe('notice 通道（守卫 → UI）', () => {
  beforeEach(() => {
    resetNoticeChannel()
  })

  it('有订阅者时立即投递', () => {
    const got: NoticeMessage[] = []
    onNotice((m) => got.push(m))
    emitNotice({ key: 'common.noPermission', severity: 'warn' })
    expect(got).toEqual([{ key: 'common.noPermission', severity: 'warn' }])
  })

  it('【关键回归】无订阅者时暂存，订阅建立后补投（刷新页面场景不丢提示）', () => {
    emitNotice({ key: 'common.noPermission', severity: 'warn' })
    emitNotice({ key: 'common.failed', severity: 'error' })

    const got: NoticeMessage[] = []
    onNotice((m) => got.push(m))
    expect(got.map((m) => m.key)).toEqual(['common.noPermission', 'common.failed'])
  })

  it('补投只发生一次（不重复推送）', () => {
    emitNotice({ key: 'common.noPermission', severity: 'warn' })
    const first: NoticeMessage[] = []
    onNotice((m) => first.push(m))
    expect(first).toHaveLength(1)

    const second: NoticeMessage[] = []
    onNotice((m) => second.push(m))
    expect(second).toHaveLength(0)
  })

  it('取消订阅后不再投递', () => {
    const got: NoticeMessage[] = []
    const off = onNotice((m) => got.push(m))
    off()
    emitNotice({ key: 'common.noPermission', severity: 'warn' })
    expect(got).toHaveLength(0)
  })
})
