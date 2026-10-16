#!/usr/bin/env node
/**
 * T1-3 首屏体积门禁（Doc/23 §3.2，问题 P2-5「无 bundle 体积门禁」）。
 *
 * 口径：`dist/index.html` 首屏需加载的**全部运行时资源**——入口脚本 + `modulepreload`
 * 预载块 + 样式表（浏览器打开页面即下载的部分）。gzip 后求和，与 design.md §11
 * 的「首屏 JS < 200KB gzip」预算比对，超出即失败（CI 红）。
 *
 * 为何含样式表而不仅是 JS：首屏总量才是用户真实付出的带宽，CSS 膨胀同样是回归；
 * 且 Doc/23 §1.2 明确判据应看「首屏加载总量」而非单个 chunk 名义大小
 * （R-2 交付后 index chunk 数值变大是分包结构变化，不是回归）。
 *
 * 为什么自研而非 size-limit：零新增依赖（仓库纪律），需求仅百行量级；
 * 预算常量与文档同源维护，见 Doc/23 §7.15。
 *
 * 用法：
 *   npm run bundle:check              # 校验默认 dist/ 与默认预算
 *   node scripts/check-bundle-size.mjs --budget=220 --dist=./dist   # 本地排查用
 */
import { gzipSync } from 'node:zlib'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

/** design.md §11：首屏 JS gzip 预算（本门禁按「首屏总量」口径执行，更严） */
export const DEFAULT_BUDGET_KB = 200

/** 首屏资源白名单：只有这些 rel 的 link 参与首屏下载 */
const FIRST_PAINT_RELS = new Set(['modulepreload', 'stylesheet'])

export class BundleBudgetError extends Error {
  constructor(message) {
    super(message)
    this.name = 'BundleBudgetError'
  }
}

/**
 * 从 index.html 提取首屏资源 URL（保持文档顺序，去重）。
 * 只认站内 `/assets/*.js|mjs|css`：外链（CDN）、`data:`、favicon 等一律不计。
 * @param {string} html
 * @returns {Array<{url: string, kind: 'script' | 'modulepreload' | 'stylesheet'}>}
 */
export function extractFirstPaintAssets(html) {
  const assets = []
  const seen = new Set()
  const tagRe = /<(?:script|link)\b[^>]*>/gi // 逐标签扫描，天然保持文档顺序
  for (const [tag] of html.matchAll(tagRe)) {
    const attr = (name) => tag.match(new RegExp(`\\b${name}="([^"]*)"`))?.[1]
    let url
    let kind
    if (/^<script/i.test(tag)) {
      url = attr('src')
      kind = 'script'
    } else {
      const rel = attr('rel')
      if (!rel || !FIRST_PAINT_RELS.has(rel.toLowerCase())) continue
      url = attr('href')
      kind = rel.toLowerCase()
    }
    if (!url || !url.startsWith('/') || seen.has(url)) continue
    if (!/\.(?:js|mjs|css)$/.test(url)) continue
    seen.add(url)
    assets.push({ url, kind })
  }
  return assets
}

/** 单文件 gzip 后大小（KB，1024 进制） */
export function gzipSizeKb(filePath) {
  return gzipSync(readFileSync(filePath)).length / 1024
}

const kb = (v) => `${v.toFixed(1)} KB`

/**
 * 量测 dist：返回每个首屏资源的 raw/gzip 体积与合计。
 * @param {string} distDir
 */
export function measureDist(distDir) {
  const indexPath = join(distDir, 'index.html')
  if (!existsSync(indexPath)) {
    throw new BundleBudgetError(
      `未找到 ${indexPath}——请先执行 \`npm run build\` 产出构建产物后再跑体积门禁`,
    )
  }
  const assets = extractFirstPaintAssets(readFileSync(indexPath, 'utf8')).map(({ url, kind }) => {
    const filePath = join(distDir, url)
    if (!existsSync(filePath)) {
      throw new BundleBudgetError(`index.html 引用的资源不存在：${url}（dist 产物不完整？）`)
    }
    return {
      url,
      kind,
      rawKb: statSync(filePath).size / 1024,
      gzipKb: gzipSizeKb(filePath),
    }
  })
  if (assets.length === 0) {
    throw new BundleBudgetError('index.html 中未解析到任何首屏资源——解析规则与 Vite 输出可能已失配')
  }
  const sum = (kind) =>
    assets.filter((a) => (kind === 'css' ? a.url.endsWith('.css') : !a.url.endsWith('.css')))
  return {
    assets,
    totalRawKb: assets.reduce((s, a) => s + a.rawKb, 0),
    totalGzipKb: assets.reduce((s, a) => s + a.gzipKb, 0),
    jsGzipKb: sum('js').reduce((s, a) => s + a.gzipKb, 0),
    cssGzipKb: sum('css').reduce((s, a) => s + a.gzipKb, 0),
  }
}

/**
 * 预算判定。
 * @param {number} totalGzipKb
 * @param {number} budgetKb
 */
export function evaluateBudget(totalGzipKb, budgetKb = DEFAULT_BUDGET_KB) {
  const overByKb = totalGzipKb - budgetKb
  return {
    ok: overByKb <= 0,
    overByKb: Math.max(overByKb, 0),
    usagePct: (totalGzipKb / budgetKb) * 100,
  }
}

function formatReport(result, evaluated, budgetKb) {
  const lines = [`首屏体积门禁（T1-3）· 预算 ${kb(budgetKb)} gzip`]
  for (const a of result.assets) {
    lines.push(
      `  ${a.url.padEnd(46)} ${kb(a.rawKb).padStart(10)} raw ${kb(a.gzipKb).padStart(10)} gzip`,
    )
  }
  const headroom = evaluated.ok
    ? `余量 ${kb(budgetKb - result.totalGzipKb)}`
    : `超出 ${kb(evaluated.overByKb)}`
  lines.push(
    `  合计 ${result.assets.length} 项 · ${kb(result.totalGzipKb)} gzip` +
      `（其中 JS ${kb(result.jsGzipKb)} + CSS ${kb(result.cssGzipKb)}）` +
      ` · 占预算 ${evaluated.usagePct.toFixed(1)}% · ${headroom}`,
  )
  return lines.join('\n')
}

/**
 * 门禁主体（可注入 dist/预算/日志，便于单测）。
 * 通过则返回量测结果；超预算抛 BundleBudgetError。
 */
export function run({ distDir, budgetKb = DEFAULT_BUDGET_KB, log = console.log } = {}) {
  const dir = distDir ?? join(dirname(fileURLToPath(import.meta.url)), '..', 'dist')
  const result = measureDist(dir)
  const evaluated = evaluateBudget(result.totalGzipKb, budgetKb)
  log(formatReport(result, evaluated, budgetKb))
  if (!evaluated.ok) {
    const worst = [...result.assets].sort((a, b) => b.gzipKb - a.gzipKb).slice(0, 3)
    throw new BundleBudgetError(
      [
        `❌ 首屏体积超预算：合计 ${kb(result.totalGzipKb)} gzip > 预算 ${kb(budgetKb)}（超出 ${kb(evaluated.overByKb)}）`,
        '   体积最大的 3 个首屏资源：',
        ...worst.map((a) => `     ${a.url}  ${kb(a.gzipKb)} gzip`),
        '   处理建议：排查本次新增依赖/代码；确需放宽预算时按 design.md §11 评审后同步本脚本常量与 Doc/23 §7.15。',
      ].join('\n'),
    )
  }
  log('✅ 首屏体积在预算内')
  return { ...result, ...evaluated, budgetKb }
}

function parseArgs(argv) {
  const opts = {}
  for (const arg of argv) {
    const budget = arg.match(/^--budget=([\d.]+)$/)
    if (budget) opts.budgetKb = Number(budget[1])
    const dist = arg.match(/^--dist=(.+)$/)
    if (dist) opts.distDir = resolve(dist[1])
  }
  return opts
}

function main() {
  try {
    run(parseArgs(process.argv.slice(2)))
  } catch (err) {
    console.error(err instanceof BundleBudgetError ? err.message : err)
    process.exitCode = 1
  }
}

// 仅在直接执行时跑 main（被 import 时不触发，保证可单测）
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()
