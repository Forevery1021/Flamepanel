/**
 * S-13 防火墙规则表单：纯函数校验 / 规范化，与后端 `domain/firewall.rs` 同口径。
 *
 * - 端口：`22` / `8000-9000`（也接受 `8000:9000`）/ 逗号分隔，≤ 15 个槽位（端口段计 2）；
 *   仅 TCP / UDP 可填端口；
 * - 地址：IPv4 / IPv6 地址或 CIDR；空 / `any` / `0.0.0.0/0` / `::/0` = 任意；
 *   主机位不为 0 的 CIDR 预览为网络地址（后端按同样规则落库）；
 * - 名称 1–64 字符、描述 ≤ 255、优先级 1–999。
 *
 * 错误以 i18n key + 参数返回，组件只负责 `t()`。
 */
import type { FirewallRule, FirewallStatus } from '@/types'

export const MAX_PORT_SLOTS = 15
export const MAX_NAME_CHARS = 64
export const MAX_DESCRIPTION_CHARS = 255
export const PRIORITY_MIN = 1
export const PRIORITY_MAX = 999
export const ANY_SOURCE = '0.0.0.0/0'

export type RuleProtocol = 'tcp' | 'udp' | 'icmp' | 'any'
export type RuleAction = 'allow' | 'deny' | 'reject'
export type RuleDirection = 'in' | 'out'

export interface FirewallRuleForm {
  name: string
  description: string
  protocol: RuleProtocol
  port: string
  source: string
  action: RuleAction
  direction: RuleDirection
  priority: number
}

export interface FieldError {
  key: string
  params?: Record<string, string | number>
}

export type FormErrors = Partial<Record<keyof FirewallRuleForm, FieldError>>

export interface PortItem {
  start: number
  end: number
}

type Result<T> = { ok: true; value: T } | { ok: false; error: FieldError }

const ok = <T>(value: T): Result<T> => ({ ok: true, value })
const fail = <T>(key: string, params?: FieldError['params']): Result<T> => ({
  ok: false,
  error: { key, params },
})

// ── 端口 ─────────────────────────────────────────────────────

function parsePort(s: string): number | null {
  if (!/^\d+$/.test(s)) return null
  const n = Number(s)
  return n >= 1 && n <= 65535 ? n : null
}

/** 解析端口表达式；空串 → `[]`（= 所有端口）。 */
export function parsePorts(raw: string): Result<PortItem[]> {
  const text = raw.trim()
  if (!text) return ok([])
  const items: PortItem[] = []
  for (const rawPart of text.split(',')) {
    const part = rawPart.trim()
    if (!part) return fail('firewall.errors.portEmptyItem')
    const pieces = part.split(/[-:]/)
    if (pieces.length > 2) return fail('firewall.errors.portInvalid', { value: part })
    const [a, b = a] = pieces.map((p) => p.trim())
    const start = parsePort(a ?? '')
    const end = parsePort(b ?? '')
    if (start === null || end === null) return fail('firewall.errors.portInvalid', { value: part })
    if (start > end) return fail('firewall.errors.portReversed', { value: part })
    if (!items.some((i) => i.start === start && i.end === end)) items.push({ start, end })
  }
  const slots = items.reduce((n, i) => n + (i.start === i.end ? 1 : 2), 0)
  if (slots > MAX_PORT_SLOTS) return fail('firewall.errors.portTooMany', { max: MAX_PORT_SLOTS })
  return ok(items)
}

/** 规范存储形态：`80,443,8000-9000`。 */
export function formatPorts(items: PortItem[]): string {
  return items.map((i) => (i.start === i.end ? `${i.start}` : `${i.start}-${i.end}`)).join(',')
}

export function portsAllowed(protocol: string): boolean {
  return protocol === 'tcp' || protocol === 'udp'
}

// ── 地址 ─────────────────────────────────────────────────────

export function isAnyAddress(raw: string): boolean {
  return ['', 'any', '0.0.0.0/0', '::/0'].includes(raw.trim().toLowerCase())
}

function parseIpv4(s: string): bigint | null {
  const parts = s.split('.')
  if (parts.length !== 4) return null
  let n = 0n
  for (const p of parts) {
    // 与 Rust `Ipv4Addr::from_str` 一致：不接受前导 0（`01`）
    if (!/^\d{1,3}$/.test(p) || (p.length > 1 && p.startsWith('0')) || Number(p) > 255) return null
    n = (n << 8n) | BigInt(p)
  }
  return n
}

function parseIpv6(s: string): bigint | null {
  if (!/^[0-9a-fA-F:.]+$/.test(s)) return null
  const halves = s.split('::')
  if (halves.length > 2) return null
  const toGroups = (part: string): number[] | null => {
    if (!part) return []
    const out: number[] = []
    const tokens = part.split(':')
    for (let i = 0; i < tokens.length; i++) {
      const tok = tokens[i]!
      if (i === tokens.length - 1 && tok.includes('.')) {
        const v4 = parseIpv4(tok)
        if (v4 === null) return null
        out.push(Number(v4 >> 16n), Number(v4 & 0xffffn))
      } else if (/^[0-9a-fA-F]{1,4}$/.test(tok)) {
        out.push(parseInt(tok, 16))
      } else {
        return null
      }
    }
    return out
  }
  const head = toGroups(halves[0]!)
  const tail = halves.length === 2 ? toGroups(halves[1]!) : []
  if (!head || !tail) return null
  const missing = 8 - head.length - tail.length
  if (halves.length === 2 ? missing < 1 : missing !== 0) return null
  const groups = [...head, ...Array<number>(Math.max(missing, 0)).fill(0), ...tail]
  return groups.reduce((n, g) => (n << 16n) | BigInt(g), 0n)
}

function formatIpv4(n: bigint): string {
  return [24n, 16n, 8n, 0n].map((s) => ((n >> s) & 0xffn).toString()).join('.')
}

/** RFC 5952 形态（最长一段连续 0 压缩为 `::`，与 Rust `Ipv6Addr` Display 一致）。 */
function formatIpv6(n: bigint): string {
  // IPv4 映射地址（::ffff:a.b.c.d）按 Rust 的写法保留点分形式
  if (n >> 32n === 0xffffn) return `::ffff:${formatIpv4(n & 0xffffffffn)}`
  const groups = Array.from({ length: 8 }, (_, i) => Number((n >> BigInt((7 - i) * 16)) & 0xffffn))
  let bestStart = -1
  let bestLen = 0
  for (let i = 0; i < 8;) {
    if (groups[i] !== 0) {
      i++
      continue
    }
    let j = i
    while (j < 8 && groups[j] === 0) j++
    if (j - i > bestLen && j - i > 1) {
      bestStart = i
      bestLen = j - i
    }
    i = j
  }
  const hex = groups.map((g) => g.toString(16))
  if (bestStart < 0) return hex.join(':')
  return `${hex.slice(0, bestStart).join(':')}::${hex.slice(bestStart + bestLen).join(':')}`
}

/**
 * 解析地址 / CIDR，返回规范形态（任意地址 → `null`）。
 * `192.168.1.10/24` → `192.168.1.0/24`；`10.0.0.5/32` → `10.0.0.5`。
 */
export function normalizeAddress(raw: string): Result<string | null> {
  const text = raw.trim()
  if (isAnyAddress(text)) return ok(null)
  const slash = text.indexOf('/')
  const addrPart = slash >= 0 ? text.slice(0, slash) : text
  const prefixPart = slash >= 0 ? text.slice(slash + 1) : null
  const v4 = parseIpv4(addrPart)
  const v6 = v4 === null ? parseIpv6(addrPart) : null
  if (v4 === null && v6 === null) return fail('firewall.errors.addressInvalid', { value: addrPart })
  const bits = v4 !== null ? 32 : 128
  let prefix = bits
  if (prefixPart !== null) {
    if (!/^\d+$/.test(prefixPart) || Number(prefixPart) > bits) {
      return fail('firewall.errors.prefixInvalid', { value: prefixPart, max: bits })
    }
    prefix = Number(prefixPart)
  }
  if (prefix === 0) return fail('firewall.errors.prefixZero')
  const value = (v4 ?? v6)!
  const mask = ((1n << BigInt(bits)) - 1n) ^ ((1n << BigInt(bits - prefix)) - 1n)
  const network = value & mask
  const addr = v4 !== null ? formatIpv4(network) : formatIpv6(network)
  return ok(prefix === bits ? addr : `${addr}/${prefix}`)
}

// ── 表单 ─────────────────────────────────────────────────────

export function emptyRuleForm(): FirewallRuleForm {
  return {
    name: '',
    description: '',
    protocol: 'tcp',
    port: '',
    source: ANY_SOURCE,
    action: 'allow',
    direction: 'in',
    priority: 50,
  }
}

export function formFromRule(rule: FirewallRule): FirewallRuleForm {
  const pick = <T extends string>(v: string, allowed: readonly T[], fallback: T): T =>
    (allowed as readonly string[]).includes(v) ? (v as T) : fallback
  return {
    name: rule.name,
    description: rule.description ?? '',
    protocol: pick(rule.protocol, ['tcp', 'udp', 'icmp', 'any'] as const, 'tcp'),
    port: rule.port ?? '',
    source: rule.source ?? ANY_SOURCE,
    action: pick(rule.action, ['allow', 'deny', 'reject'] as const, 'allow'),
    direction: pick(rule.direction, ['in', 'out'] as const, 'in'),
    priority: rule.priority,
  }
}

/** 校验整张表单，返回全部字段错误（空对象 = 通过）。 */
export function validateRuleForm(form: FirewallRuleForm): FormErrors {
  const errors: FormErrors = {}
  const name = form.name.trim()
  if (!name) errors.name = { key: 'common.required' }
  else if ([...name].length > MAX_NAME_CHARS) {
    errors.name = { key: 'firewall.errors.tooLong', params: { max: MAX_NAME_CHARS } }
  }
  if ([...form.description.trim()].length > MAX_DESCRIPTION_CHARS) {
    errors.description = { key: 'firewall.errors.tooLong', params: { max: MAX_DESCRIPTION_CHARS } }
  }
  const ports = parsePorts(form.port)
  if (!ports.ok) errors.port = ports.error
  else if (ports.value.length && !portsAllowed(form.protocol)) {
    errors.port = { key: 'firewall.errors.portNeedsProto' }
  }
  const source = normalizeAddress(form.source)
  if (!source.ok) errors.source = source.error
  if (
    !Number.isInteger(form.priority) ||
    form.priority < PRIORITY_MIN ||
    form.priority > PRIORITY_MAX
  ) {
    errors.priority = {
      key: 'firewall.errors.priorityRange',
      params: { min: PRIORITY_MIN, max: PRIORITY_MAX },
    }
  }
  return errors
}

/**
 * 请求体（已规范化）。端口 / 描述始终发送——空串表示清空（后端规范为 null），
 * 修复了编辑时无法清空端口的问题。
 */
export function toRulePayload(form: FirewallRuleForm): Record<string, string | number> {
  const ports = parsePorts(form.port)
  const source = normalizeAddress(form.source)
  return {
    name: form.name.trim(),
    description: form.description.trim(),
    protocol: form.protocol,
    port: portsAllowed(form.protocol) && ports.ok ? formatPorts(ports.value) : '',
    source: source.ok ? (source.value ?? ANY_SOURCE) : form.source.trim(),
    action: form.action,
    direction: form.direction,
    priority: form.priority,
  }
}

/**
 * 当前防火墙后端对该组合的限制（提前提示，免得保存后才从系统命令报错）：
 * firewalld rich rule 只管入站；ufw 命令行不支持 ICMP。
 */
export function backendLimitation(
  backend: string | null | undefined,
  form: Pick<FirewallRuleForm, 'protocol' | 'direction'>,
): string | null {
  const b = (backend ?? '').toLowerCase()
  if (b === 'firewalld' && form.direction === 'out') return 'firewall.limits.firewalldOutbound'
  if (b === 'ufw' && form.protocol === 'icmp') return 'firewall.limits.ufwIcmp'
  return null
}

/**
 * 解读 `GET /api/firewall/status`：先判"关"——`inactive` 含 `active`、firewalld 的
 * `not running` 含 `running`（修复此前把未启用的 ufw 显示为已启用的缺陷）。
 * 状态未知时按"已启用"处理（与既有行为一致：按钮显示"关闭"）。
 */
export function firewallStatusView(status: FirewallStatus | null | undefined): {
  enabled: boolean
  severity: 'success' | 'warning' | 'danger' | 'info'
  label: string | null
} {
  if (!status) return { enabled: true, severity: 'info', label: null }
  const text = (status.status || '').toLowerCase()
  const off = /inactive|disable|not running/.test(text)
  const on = !off && /active|running|enable/.test(text)
  return {
    enabled: on,
    severity: off ? 'danger' : on ? 'success' : 'warning',
    label: `${(status.backend_name || '').toUpperCase()} - ${(status.status || '').substring(0, 40)}`,
  }
}
