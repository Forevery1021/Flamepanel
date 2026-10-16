/**
 * 密码强度估算（安装向导 / 修改密码共用；纯函数，可单测）。
 *
 * 不追求 zxcvbn 级别的字典分析，只做运维面板足够的启发式：
 * - 长度是首要因素（8 起步，12 / 16 各加一档）；
 * - 字符类别（小写 / 大写 / 数字 / 符号）每多一类加分；
 * - 明显弱模式扣分：纯数字、连续重复、键盘顺子、常见弱口令。
 *
 * 返回 0–4：0 太短（< 8）、1 弱、2 一般、3 强、4 很强。
 */
export type PasswordScore = 0 | 1 | 2 | 3 | 4

const COMMON = new Set([
  'password',
  'password1',
  '12345678',
  '123456789',
  'qwertyui',
  'qwerty123',
  'admin123',
  'admin1234',
  'iloveyou',
  'letmein1',
  'flamepanel',
])

const SEQUENCES = ['0123456789', 'abcdefghijklmnopqrstuvwxyz', 'qwertyuiop', 'asdfghjkl', 'zxcvbnm']

function hasSequence(pw: string, len = 4): boolean {
  const lower = pw.toLowerCase()
  for (const seq of SEQUENCES) {
    const rev = [...seq].reverse().join('')
    for (let i = 0; i + len <= seq.length; i++) {
      if (lower.includes(seq.slice(i, i + len)) || lower.includes(rev.slice(i, i + len)))
        return true
    }
  }
  return false
}

export function passwordScore(pw: string): PasswordScore {
  if (pw.length < 8) return 0
  if (COMMON.has(pw.toLowerCase())) return 1

  let points = 0
  if (pw.length >= 12) points += 1
  if (pw.length >= 16) points += 1

  let classes = 0
  if (/[a-z]/.test(pw)) classes += 1
  if (/[A-Z]/.test(pw)) classes += 1
  if (/\d/.test(pw)) classes += 1
  if (/[^A-Za-z0-9]/.test(pw)) classes += 1
  points += Math.max(0, classes - 1)

  if (/^(.)\1+$/.test(pw))
    points -= 2 // 全部同一字符
  else if (/(.)\1{2,}/.test(pw)) points -= 1 // 连续三个以上重复
  if (/^\d+$/.test(pw)) points -= 1 // 纯数字
  if (hasSequence(pw)) points -= 1 // 顺子

  const score = Math.max(1, Math.min(4, 1 + points))
  return score as PasswordScore
}

/** 强度档位 → i18n key 后缀（调用方拼 `setup.strength.<key>`） */
export const PASSWORD_SCORE_KEYS: Record<PasswordScore, string> = {
  0: 'tooShort',
  1: 'weak',
  2: 'fair',
  3: 'strong',
  4: 'veryStrong',
}
