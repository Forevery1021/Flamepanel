/**
 * WASM 插件宿主能力（Doc/21 §5.7）的展示辅助：风险分级 + i18n 文案键。
 * 校验以后端为准（敏感目录等），这里只做展示与输入整理。
 */

export type CapabilitySeverity = 'danger' | 'warning' | 'info'

export interface CapabilityView {
  raw: string
  severity: CapabilitySeverity
  /** i18n 键（plugin.capDesc.*） */
  key: string
  params: Record<string, string>
}

export function describeCapability(raw: string): CapabilityView {
  if (raw.startsWith('fs:write:')) {
    return {
      raw,
      severity: 'danger',
      key: 'plugin.capDesc.fsWrite',
      params: { path: raw.slice(9) },
    }
  }
  if (raw.startsWith('fs:read:')) {
    return {
      raw,
      severity: 'warning',
      key: 'plugin.capDesc.fsRead',
      params: { path: raw.slice(8) },
    }
  }
  if (raw === 'metrics:read') {
    return { raw, severity: 'info', key: 'plugin.capDesc.metricsRead', params: {} }
  }
  if (raw === 'settings:write') {
    return { raw, severity: 'info', key: 'plugin.capDesc.settingsWrite', params: {} }
  }
  return { raw, severity: 'danger', key: 'plugin.capDesc.unknown', params: { cap: raw } }
}

/** 文本框（每行 / 逗号分隔）→ 去空去重的能力列表 */
export function parseCapabilityInput(text: string): string[] {
  const out: string[] = []
  for (const item of text.split(/[\n,]/)) {
    const v = item.trim()
    if (v && !out.includes(v)) out.push(v)
  }
  return out
}
