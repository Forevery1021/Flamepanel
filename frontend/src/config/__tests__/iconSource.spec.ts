import { describe, it, expect } from 'vitest'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { createGenerator } from 'unocss'
import config from '../../../uno.config'

/**
 * T0-3 图标源本地化回归（Doc/23 §3.1 / 问题 P0-4）。
 *
 * 历史配置在 `uno.config.ts` 启用了 `presetIcons({ cdn: 'https://esm.sh/' })`：
 * 只要源码里出现 `i-*` 图标类，构建期就会去 esm.sh 抓远程图标集 JSON
 * ——**断外网/内网环境下 `npm run build` 直接失败**（实测
 * `FetchError: [GET] https://esm.sh/@iconify-json/mdi/icons.json`），且把第三方
 * CDN 变成构建期依赖（供应链风险）。
 *
 * 本仓图标唯一来源是 `@openvue/openicons` 字体图标（`oi-*`，随包本地分发），
 * 源码中 `i-*` 图标类使用量为 0 —— 故直接移除 presetIcons，且不新增任何依赖。
 *
 * 验收口径（与任务单逐条对应）：
 * 1. `npm run build` 产物无 esm.sh 外链；
 * 2. 断外网环境图标正常（图标走本地字体，构建不触网）。
 */

const frontendRoot = resolve(import.meta.dirname, '../../..')

/**
 * 递归收集源码文件。
 *
 * 跳过 `node_modules` / `dist`（构建产物）与 `__tests__`——测试文件会把
 * `i-*`、CDN 字面量当作**输入**或写在注释里，不代表应用代码的真实用法。
 */
function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) {
      if (name === 'node_modules' || name === 'dist' || name === '__tests__') continue
      collectSourceFiles(full, out)
    } else if (/\.(vue|ts|tsx|js|jsx|css|html)$/.test(name)) {
      out.push(full)
    }
  }
  return out
}

/** 剥掉行注释与块注释（配置文件的注释会以历史说明形式提到 esm.sh） */
function stripComments(text: string): string {
  return text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1')
}

describe('T0-3 图标源本地化 · 构建不触网（P0-4）', () => {
  it('【核心回归】uno.config 不再启用 presetIcons（关闭构建期远程抓取通道）', () => {
    const presets = (config.presets ?? []) as Array<{ name?: string }>
    expect(presets.map((p) => p?.name)).not.toContain('@unocss/preset-icons')
  })

  it('【核心回归】i-* 图标类不再产出图标规则，产物无 esm.sh 痕迹', async () => {
    const uno = await createGenerator(config as never)
    const { css } = await uno.generate('i-mdi-home i-carbon-sun', { preflights: false })
    // presetIcons 移除后 `i-*` 不再是图标工具类：既不产出图标规则，也无从外联 esm.sh
    expect(css).not.toContain('un-icon')
    expect(css).not.toContain('esm.sh')
  })

  it('源码不混用非 openicons 图标：不存在 i-* 图标类（防「静默无图标」）', () => {
    const offenders: string[] = []
    for (const file of collectSourceFiles(join(frontendRoot, 'src'))) {
      const text = readFileSync(file, 'utf-8')
      // 前导边界排除 oi- / pi- / api- 等包含 "i-" 的既有命名
      for (const m of text.matchAll(/(?<![\w-])i-[a-z0-9]+(?:-[a-z0-9]+)*/g)) {
        offenders.push(`${file.replace(`${frontendRoot}/`, '')}: ${m[0]}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('应用源码与 Uno 配置无 esm.sh 等远程 CDN 引用（离网部署安全）', () => {
    const files = [
      ...collectSourceFiles(join(frontendRoot, 'src')),
      join(frontendRoot, 'uno.config.ts'),
    ]
    const offenders = files.filter((f) =>
      stripComments(readFileSync(f, 'utf-8')).includes('esm.sh'),
    )
    expect(offenders).toEqual([])
  })
})
