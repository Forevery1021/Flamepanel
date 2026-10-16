import { describe, it, expect } from 'vitest'
import { formatSize } from '@/utils/format'

describe('formatSize（统一体积格式化）', () => {
  it('0 → "0 B"', () => {
    expect(formatSize(0)).toBe('0 B')
  })

  it('字节级不带小数', () => {
    expect(formatSize(512)).toBe('512 B')
  })

  it('KB 保留 1 位小数', () => {
    expect(formatSize(1536)).toBe('1.5 KB')
  })

  it('MB 保留 1 位小数', () => {
    expect(formatSize(1024 * 1024 * 2.5)).toBe('2.5 MB')
  })

  it('整数 KB/MB 省略 .0', () => {
    expect(formatSize(2048)).toBe('2 KB')
    expect(formatSize(1024 * 1024 * 3)).toBe('3 MB')
  })

  it('GB/TB 进位正确', () => {
    expect(formatSize(1024 ** 3)).toBe('1 GB')
    expect(formatSize(1024 ** 4)).toBe('1 TB')
  })

  it('超大值进位到 TB', () => {
    expect(formatSize(1024 ** 4 * 1.5)).toBe('1.5 TB')
  })
})
