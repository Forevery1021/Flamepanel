import { describe, it, expect } from 'vitest'
import {
  cpuTone,
  formatCpu,
  formatRunTime,
  killErrorKey,
  memoryShare,
  normalizeState,
  protectedI18nKey,
  signalI18nKey,
  stateTone,
  userOptions,
} from '@/utils/processes'

/** S-12 进程管理纯函数 */
describe('utils/processes', () => {
  it('normalizeState 归一 sysinfo 状态名，未知值落到 other', () => {
    expect(normalizeState('Run')).toBe('run')
    expect(normalizeState('Sleep')).toBe('sleep')
    expect(normalizeState('Idle')).toBe('idle')
    expect(normalizeState('UninterruptibleDiskSleep')).toBe('disk_sleep')
    expect(normalizeState('Zombie')).toBe('zombie')
    expect(normalizeState('Stop')).toBe('stop')
    expect(normalizeState('Tracing')).toBe('tracing')
    expect(normalizeState('Dead')).toBe('dead')
    expect(normalizeState('Unknown(0)')).toBe('other')
  })

  it('stateTone：运行绿、僵尸 / 退出红、暂停 / 不可中断黄、其余中性', () => {
    expect(stateTone('run')).toBe('success')
    expect(stateTone('zombie')).toBe('danger')
    expect(stateTone('dead')).toBe('danger')
    expect(stateTone('disk_sleep')).toBe('warning')
    expect(stateTone('stop')).toBe('warning')
    expect(stateTone('sleep')).toBe('neutral')
    expect(stateTone('other')).toBe('neutral')
  })

  it('cpuTone / formatCpu：阈值 40 / 80，小值 1 位小数', () => {
    expect(cpuTone(5)).toBe('neutral')
    expect(cpuTone(40)).toBe('warning')
    expect(cpuTone(180)).toBe('danger')
    expect(formatCpu(0)).toBe('0')
    expect(formatCpu(Number.NaN)).toBe('0')
    expect(formatCpu(3.14159)).toBe('3.1')
    expect(formatCpu(55.6)).toBe('56')
  })

  it('memoryShare 夹在 0–100，合计为 0 时为 0', () => {
    expect(memoryShare(50, 200)).toBe(25)
    expect(memoryShare(500, 200)).toBe(100)
    expect(memoryShare(10, 0)).toBe(0)
    expect(memoryShare(0, 100)).toBe(0)
  })

  it('formatRunTime 基于 started_at 计算运行时长，缺失与未来时间安全', () => {
    const now = 1_700_000_000_000
    const started = now / 1000 - (2 * 86_400 + 3 * 3_600)
    expect(formatRunTime(started, now)).toBe('2d 3h')
    expect(formatRunTime(now / 1000 - 125, now)).toBe('2m')
    expect(formatRunTime(now / 1000 + 60, now)).toBe('0m')
    expect(formatRunTime(0, now)).toBe('—')
  })

  it('i18n key 映射', () => {
    expect(protectedI18nKey('panel_ancestor')).toBe('processes.protected.panel_ancestor')
    expect(signalI18nKey('KILL')).toBe('processes.signal.KILL')
  })

  it('userOptions 去重排序，并保留已选但当前页不存在的用户', () => {
    const rows = [{ user: 'www-data' }, { user: 'root' }, { user: null }, { user: 'root' }]
    expect(userOptions(rows)).toEqual(['root', 'www-data'])
    expect(userOptions(rows, 'postgres')).toEqual(['postgres', 'root', 'www-data'])
  })

  it('killErrorKey：404 / 409 / 400 / 权限不足各有专用提示，其余交给通用文案', () => {
    expect(killErrorKey({ status: 404 })).toBe('processes.kill.gone')
    expect(killErrorKey({ status: 409 })).toBe('processes.kill.reused')
    expect(killErrorKey({ status: 400 })).toBe('processes.kill.protected')
    expect(
      killErrorKey({
        status: 403,
        message: 'Forbidden: kill 12: insufficient privilege (panel runs as non-root)',
      }),
    ).toBe('processes.kill.privilege')
    expect(killErrorKey({ status: 403, message: 'Forbidden: missing toolbox:execute' })).toBeNull()
    // 5xx message 已被后端脱敏，不再尝试匹配
    expect(killErrorKey({ status: 500, message: 'insufficient privilege' })).toBeNull()
    expect(killErrorKey(null)).toBeNull()
  })
})
