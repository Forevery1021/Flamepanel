import { describe, it, expect } from 'vitest'
import { passwordScore } from '@/utils/password'

describe('passwordScore · 启发式密码强度', () => {
  it('短于 8 位 = 0（无论内容）', () => {
    expect(passwordScore('')).toBe(0)
    expect(passwordScore('Ab1!Ab1')).toBe(0)
  })

  it('常见弱口令 / 纯数字 / 顺子 / 重复只能是弱', () => {
    expect(passwordScore('password')).toBe(1)
    expect(passwordScore('12345678')).toBe(1)
    expect(passwordScore('aaaaaaaa')).toBe(1)
    expect(passwordScore('qwertyuiop')).toBe(1)
  })

  it('长度与字符类别共同提升档位', () => {
    expect(passwordScore('flamepnl')).toBe(1) // 8 位单类
    expect(passwordScore('Flamepnl9')).toBe(3) // 三类
    expect(passwordScore('Fl@mePanel-2026!')).toBe(4) // 16 位四类
  })

  it('结果单调：更长更杂不会降档', () => {
    const a = passwordScore('Panel2026')
    const b = passwordScore('Panel2026!!xx')
    expect(b).toBeGreaterThanOrEqual(a)
  })
})
