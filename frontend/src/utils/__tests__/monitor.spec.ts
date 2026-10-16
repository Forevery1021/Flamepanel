import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { reportError, flushErrors } from '@/utils/monitor'

/**
 * R-3 回归：首次错误必须立即输出，去重只作用于之后的重复错误。
 * （修复前：首错被登记分支的提前 return 吞掉，只有第二次起才可见。）
 */
describe('reportError 去重与首错输出（R-3）', () => {
  let errSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    flushErrors()
    errSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    errSpy.mockRestore()
  })

  it('首次错误立即输出', () => {
    reportError(new Error('boom-1'), { source: 'render' })
    expect(errSpy).toHaveBeenCalledTimes(1)
    expect(errSpy).toHaveBeenCalledWith(
      '[flamepanel:render]',
      expect.objectContaining({ message: 'boom-1', count: 1 }),
    )
  })

  it('窗口内相同错误被节流，只有首条可见', () => {
    reportError(new Error('boom-2'), { source: 'render' })
    reportError(new Error('boom-2'), { source: 'render' })
    reportError(new Error('boom-2'), { source: 'render' })
    expect(errSpy).toHaveBeenCalledTimes(1)
  })

  it('不同 source 或不同 message 各自独立输出', () => {
    reportError(new Error('same'), { source: 'render' })
    reportError(new Error('same'), { source: 'query' })
    reportError(new Error('other'), { source: 'render' })
    expect(errSpy).toHaveBeenCalledTimes(3)
  })

  it('非 Error 值转字符串输出（首错同样可见）', () => {
    reportError('plain-string', { source: 'unhandled' })
    expect(errSpy).toHaveBeenCalledTimes(1)
    expect(errSpy).toHaveBeenCalledWith(
      '[flamepanel:unhandled]',
      expect.objectContaining({ message: 'plain-string' }),
    )
  })

  it('flushErrors 后同一错误重新视为首错并输出', () => {
    reportError(new Error('boom-3'), { source: 'render' })
    flushErrors()
    reportError(new Error('boom-3'), { source: 'render' })
    expect(errSpy).toHaveBeenCalledTimes(2)
  })

  it('携带 context 时透传上下文，不输出时不含该字段', () => {
    reportError(new Error('with-ctx'), { source: 'query', context: { queryKey: ['a'] } })
    expect(errSpy).toHaveBeenCalledWith(
      '[flamepanel:query]',
      expect.objectContaining({ context: { queryKey: ['a'] } }),
    )
    reportError(new Error('no-ctx'), { source: 'query' })
    expect(errSpy).toHaveBeenLastCalledWith(
      '[flamepanel:query]',
      expect.not.objectContaining({ context: expect.anything() }),
    )
  })
})
