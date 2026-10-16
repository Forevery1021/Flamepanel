import { describe, it, expect } from 'vitest'
import {
  backendLimitation,
  emptyRuleForm,
  firewallStatusView,
  formFromRule,
  formatPorts,
  normalizeAddress,
  parsePorts,
  toRulePayload,
  validateRuleForm,
  type FirewallRuleForm,
} from '@/utils/firewallRule'
import type { FirewallRule } from '@/types'

/** S-13 防火墙规则表单纯函数：与后端 `domain/firewall.rs` 同口径。 */

function form(over: Partial<FirewallRuleForm> = {}): FirewallRuleForm {
  return { ...emptyRuleForm(), name: 'web', port: '80', ...over }
}

describe('parsePorts / formatPorts', () => {
  it('接受单端口、端口段（- 或 :）、逗号列表，去重并规范为 -', () => {
    const r = parsePorts(' 443 , 80,8000:9000, 443 ')
    expect(r.ok && formatPorts(r.value)).toBe('443,80,8000-9000')
    expect(parsePorts('')).toEqual({ ok: true, value: [] })
  })

  it('拒绝越界、非数字、颠倒、空项与超过 15 个槽位', () => {
    const key = (raw: string) => {
      const r = parsePorts(raw)
      return r.ok ? null : r.error.key
    }
    expect(key('0')).toBe('firewall.errors.portInvalid')
    expect(key('65536')).toBe('firewall.errors.portInvalid')
    expect(key('+80')).toBe('firewall.errors.portInvalid')
    expect(key('1-2-3')).toBe('firewall.errors.portInvalid')
    expect(key('9000-8000')).toBe('firewall.errors.portReversed')
    expect(key('80,')).toBe('firewall.errors.portEmptyItem')
    const list = (n: number) => Array.from({ length: n }, (_, i) => i + 1).join(',')
    expect(key(list(15))).toBeNull()
    expect(key(list(16))).toBe('firewall.errors.portTooMany')
    // 8 个端口段 = 16 槽位
    expect(key(Array.from({ length: 8 }, (_, i) => `${i + 1}000-${i + 1}001`).join(','))).toBe(
      'firewall.errors.portTooMany',
    )
  })
})

describe('normalizeAddress', () => {
  const norm = (raw: string) => {
    const r = normalizeAddress(raw)
    return r.ok ? r.value : `ERR:${r.error.key}`
  }

  it('任意地址 → null；主机位归零；/32、/128 省略前缀', () => {
    expect(norm('')).toBeNull()
    expect(norm(' ANY ')).toBeNull()
    expect(norm('0.0.0.0/0')).toBeNull()
    expect(norm('::/0')).toBeNull()
    expect(norm('192.168.1.10/24')).toBe('192.168.1.0/24')
    expect(norm('10.0.0.5/32')).toBe('10.0.0.5')
    expect(norm('10.0.0.5')).toBe('10.0.0.5')
  })

  it('IPv6：解析 :: 与内嵌 IPv4，按 RFC 5952 压缩输出', () => {
    expect(norm('2001:db8::1/48')).toBe('2001:db8::/48')
    expect(norm('2001:0DB8:0000:0000:0000:0000:0000:0001')).toBe('2001:db8::1')
    expect(norm('::ffff:192.168.1.1')).toBe('::ffff:192.168.1.1')
    expect(norm('::ffff:c0a8:101/120')).toBe('::ffff:192.168.1.0/120')
    expect(norm('fe80::1/128')).toBe('fe80::1')
    expect(norm('1:0:0:2:0:0:0:3')).toBe('1:0:0:2::3')
  })

  it('拒绝非法地址、前缀与注入', () => {
    expect(norm('1.2.3')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('256.1.1.1')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('01.2.3.4')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('1::2::3')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('1:2:3:4:5:6:7:8:9')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('example.com')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('1.2.3.4" accept')).toBe('ERR:firewall.errors.addressInvalid')
    expect(norm('10.0.0.0/33')).toBe('ERR:firewall.errors.prefixInvalid')
    expect(norm('10.0.0.0/')).toBe('ERR:firewall.errors.prefixInvalid')
    expect(norm('1.2.3.4/0')).toBe('ERR:firewall.errors.prefixZero')
  })
})

describe('validateRuleForm / toRulePayload', () => {
  it('合法表单无错误；请求体规范化且始终携带 port / description（空串 = 清空）', () => {
    const f = form({
      name: '  web ',
      port: '443, 8000:9000',
      source: '192.168.1.10/24',
      description: '  ',
    })
    expect(validateRuleForm(f)).toEqual({})
    expect(toRulePayload(f)).toEqual({
      name: 'web',
      description: '',
      protocol: 'tcp',
      port: '443,8000-9000',
      source: '192.168.1.0/24',
      action: 'allow',
      direction: 'in',
      priority: 50,
    })
    expect(toRulePayload(form({ port: '', source: 'any' }))).toMatchObject({
      port: '',
      source: '0.0.0.0/0',
    })
  })

  it('一次返回全部字段错误；ICMP / 任意协议带端口报错', () => {
    const errors = validateRuleForm(
      form({ name: ' ', port: '70000', source: 'x', priority: 0, description: 'd'.repeat(256) }),
    )
    expect(Object.keys(errors).sort()).toEqual(
      ['description', 'name', 'port', 'priority', 'source'].sort(),
    )
    expect(errors.name?.key).toBe('common.required')
    expect(errors.priority?.params).toEqual({ min: 1, max: 999 })
    expect(validateRuleForm(form({ protocol: 'icmp' })).port?.key).toBe(
      'firewall.errors.portNeedsProto',
    )
    expect(validateRuleForm(form({ protocol: 'any', port: '' }))).toEqual({})
    expect(validateRuleForm(form({ name: 'x'.repeat(65) })).name?.params).toEqual({ max: 64 })
  })

  it('formFromRule 回填编辑表单，未知取值回落缺省', () => {
    const rule = {
      id: 1,
      name: 'dns',
      description: null,
      protocol: 'udp',
      port: '53',
      source: null,
      destination: null,
      action: 'weird',
      enabled: true,
      priority: 5,
      direction: 'out',
      created_at: '',
      updated_at: '',
    } as FirewallRule
    expect(formFromRule(rule)).toEqual({
      name: 'dns',
      description: '',
      protocol: 'udp',
      port: '53',
      source: '0.0.0.0/0',
      action: 'allow',
      direction: 'out',
      priority: 5,
    })
  })
})

describe('backendLimitation / firewallStatusView', () => {
  it('firewalld 出站、ufw ICMP 给出提前提示', () => {
    expect(backendLimitation('firewalld', { protocol: 'tcp', direction: 'out' })).toBe(
      'firewall.limits.firewalldOutbound',
    )
    expect(backendLimitation('ufw', { protocol: 'icmp', direction: 'in' })).toBe(
      'firewall.limits.ufwIcmp',
    )
    expect(backendLimitation('iptables', { protocol: 'icmp', direction: 'out' })).toBeNull()
    expect(backendLimitation(null, { protocol: 'tcp', direction: 'out' })).toBeNull()
  })

  it('先判「关」：inactive / not running 不再被当成已启用', () => {
    const view = (status: string, backend_name = 'ufw') =>
      firewallStatusView({ backend: '', backend_name, status })
    expect(view('Status: inactive')).toMatchObject({ enabled: false, severity: 'danger' })
    expect(view('not running', 'firewalld')).toMatchObject({ enabled: false, severity: 'danger' })
    expect(view('Status: active')).toMatchObject({
      enabled: true,
      severity: 'success',
      label: 'UFW - Status: active',
    })
    expect(view('unknown')).toMatchObject({ enabled: false, severity: 'warning' })
    expect(firewallStatusView(null)).toEqual({ enabled: true, severity: 'info', label: null })
  })
})
