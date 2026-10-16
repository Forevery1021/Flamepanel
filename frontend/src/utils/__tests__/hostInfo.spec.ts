import { describe, it, expect } from 'vitest'
import type { HostInterface } from '@/api/toolbox'
import {
  CLOCK_SKEW_WARN_SECS,
  clockSkewSeconds,
  diskUsedPercent,
  formatLoad,
  isClockSkewed,
  usageTone,
  usedPercent,
  visibleInterfaces,
} from '@/utils/hostInfo'

function iface(name: string, virtual_iface: boolean): HostInterface {
  return { name, mac: null, addresses: [], rx_bytes: 0, tx_bytes: 0, virtual_iface }
}

describe('hostInfo utils', () => {
  it('usedPercent：取整、total 为 0 返回 0、available 超过 total 视为 0 已用', () => {
    expect(usedPercent(100, 25)).toBe(75)
    expect(usedPercent(3, 2)).toBe(33)
    expect(usedPercent(0, 0)).toBe(0)
    expect(usedPercent(100, 150)).toBe(0)
    expect(usedPercent(100, 0)).toBe(100)
  })

  it('usageTone 阈值', () => {
    expect(usageTone(10)).toBe('success')
    expect(usageTone(75)).toBe('warning')
    expect(usageTone(89)).toBe('warning')
    expect(usageTone(90)).toBe('danger')
  })

  it('clockSkewSeconds：服务器快为正、慢为负、非法时间为 0', () => {
    const t = Date.parse('2026-10-01T00:00:00Z')
    expect(clockSkewSeconds('2026-10-01T00:00:45Z', t)).toBe(45)
    expect(clockSkewSeconds('2026-09-30T23:59:00Z', t)).toBe(-60)
    expect(clockSkewSeconds('garbage', t)).toBe(0)
  })

  it('isClockSkewed 以阈值为界（含符号）', () => {
    expect(isClockSkewed(CLOCK_SKEW_WARN_SECS)).toBe(false)
    expect(isClockSkewed(CLOCK_SKEW_WARN_SECS + 1)).toBe(true)
    expect(isClockSkewed(-(CLOCK_SKEW_WARN_SECS + 1))).toBe(true)
  })

  it('visibleInterfaces 默认隐藏虚拟网卡', () => {
    const list = [iface('eth0', false), iface('lo', true), iface('veth1', true)]
    expect(visibleInterfaces(list, false).map((i) => i.name)).toEqual(['eth0'])
    expect(visibleInterfaces(list, true)).toHaveLength(3)
  })

  it('diskUsedPercent 与 formatLoad', () => {
    expect(diskUsedPercent({ total_bytes: 200, available_bytes: 50 })).toBe(75)
    expect(formatLoad([0.1, 1, 2.346])).toBe('0.10 / 1.00 / 2.35')
    expect(formatLoad([])).toBe('—')
  })
})
