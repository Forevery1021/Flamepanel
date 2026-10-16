import { describe, it, expect } from 'vitest'
import { describeCapability, parseCapabilityInput } from '@/utils/pluginCapabilities'

describe('pluginCapabilities', () => {
  it('按风险分级并提取参数', () => {
    expect(describeCapability('fs:write:/opt/data')).toMatchObject({
      severity: 'danger',
      key: 'plugin.capDesc.fsWrite',
      params: { path: '/opt/data' },
    })
    expect(describeCapability('fs:read:/var/log')).toMatchObject({
      severity: 'warning',
      params: { path: '/var/log' },
    })
    expect(describeCapability('metrics:read').severity).toBe('info')
    expect(describeCapability('settings:write').key).toBe('plugin.capDesc.settingsWrite')
    expect(describeCapability('exec:*')).toMatchObject({
      severity: 'danger',
      params: { cap: 'exec:*' },
    })
  })

  it('解析多行 / 逗号输入并去重去空', () => {
    expect(parseCapabilityInput(' metrics:read \n\nfs:read:/var/log, metrics:read ,')).toEqual([
      'metrics:read',
      'fs:read:/var/log',
    ])
    expect(parseCapabilityInput('')).toEqual([])
  })
})
