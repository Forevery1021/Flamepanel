/**
 * 体积门禁单测（T1-3，Doc/23 §3.2）。
 *
 * 门禁脚本自身也是"用户代码"——它算错会让 CI 假绿/假红，故用真实临时目录构造
 * dist 夹具做端到端验证（.mjs 纯 Node，无 jsdom 依赖，与源码单测同一 vitest 出口）。
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { gzipSync } from 'node:zlib'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  BundleBudgetError,
  DEFAULT_BUDGET_KB,
  evaluateBudget,
  extractFirstPaintAssets,
  gzipSizeKb,
  measureDist,
  run,
} from '../check-bundle-size.mjs'

/** 构造一个 dist 夹具：index.html + 若干净文件 */
function makeDist({ html, files = {} }) {
  const dir = mkdtempSync(join(tmpdir(), 'bundle-gate-'))
  mkdirSync(join(dir, 'assets'), { recursive: true })
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(dir, 'assets', name), content)
  }
  writeFileSync(join(dir, 'index.html'), html)
  return dir
}

/** 生成能被 gzip 到指定大致体积的可压缩文本（重复字符压缩率极高，故用随机串） */
function blobOfSize(bytes) {
  let s = ''
  while (s.length < bytes) s += Math.random().toString(36).slice(2)
  return s.slice(0, bytes)
}

const indexHtml = (refs) =>
  `<!DOCTYPE html><html><head>${refs.join('')}</head><body><div id="app"></div></body></html>`

let dirs = []
const track = (dir) => {
  dirs.push(dir)
  return dir
}
beforeEach(() => {
  dirs = []
})
afterEach(() => {
  for (const dir of dirs) rmSync(dir, { recursive: true, force: true })
})

describe('extractFirstPaintAssets：首屏资源解析', () => {
  it('提取入口脚本 / modulepreload / 样式表，保持文档顺序', () => {
    const html = indexHtml([
      '<script type="module" crossorigin src="/assets/index-abc.js"></script>',
      '<link rel="modulepreload" crossorigin href="/assets/vendor-abc.js">',
      '<link rel="stylesheet" crossorigin href="/assets/index-abc.css">',
    ])
    expect(extractFirstPaintAssets(html)).toEqual([
      { url: '/assets/index-abc.js', kind: 'script' },
      { url: '/assets/vendor-abc.js', kind: 'modulepreload' },
      { url: '/assets/index-abc.css', kind: 'stylesheet' },
    ])
  })

  it('忽略外链 CDN、data:、favicon 与非首屏 rel（no CDN 外联是 T0-3 的既有红线）', () => {
    const html = indexHtml([
      '<link rel="icon" type="image/svg+xml" href="/vite.svg" />',
      '<link rel="preconnect" href="https://esm.sh/">',
      '<link rel="stylesheet" href="https://cdn.example.com/a.css">',
      '<script src="https://cdn.example.com/a.js"></script>',
      '<script type="module" src="/assets/ok.js"></script>',
    ])
    expect(extractFirstPaintAssets(html)).toEqual([{ url: '/assets/ok.js', kind: 'script' }])
  })

  it('同一资源重复引用只计一次；非 js/css 后缀不计入', () => {
    const html = indexHtml([
      '<script type="module" src="/assets/dup.js"></script>',
      '<link rel="modulepreload" href="/assets/dup.js">',
      '<link rel="preload" as="font" href="/assets/font.woff2">',
    ])
    expect(extractFirstPaintAssets(html)).toEqual([{ url: '/assets/dup.js', kind: 'script' }])
  })

  it('无 <script>/<link> 时返回空数组（由 measureDist 兜底报错）', () => {
    expect(extractFirstPaintAssets('<html><head></head></html>')).toEqual([])
  })
})

describe('gzipSizeKb / evaluateBudget：度量与判定', () => {
  it('gzip 体积与 zlib 直接结果一致（同一压缩口径，避免第三套数字）', () => {
    const dir = track(makeDist({ html: indexHtml([]), files: { 'a.js': blobOfSize(50_000) } }))
    const p = join(dir, 'assets', 'a.js')
    expect(gzipSizeKb(p)).toBeCloseTo(gzipSync(readFileSync(p)).length / 1024, 5)
  })

  it('恰好等于预算算通过（边界），超出即失败并给出超出量', () => {
    expect(evaluateBudget(DEFAULT_BUDGET_KB, DEFAULT_BUDGET_KB)).toMatchObject({
      ok: true,
      overByKb: 0,
    })
    const over = evaluateBudget(260, 200)
    expect(over.ok).toBe(false)
    expect(over.overByKb).toBeCloseTo(60, 5)
    expect(over.usagePct).toBeCloseTo(130, 5)
  })

  it('预算内可算出占用比与余量（100KB / 200KB → 50%）', () => {
    const result = evaluateBudget(100, DEFAULT_BUDGET_KB)
    expect(result.ok).toBe(true)
    expect(result.usagePct).toBeCloseTo(50, 5)
  })
})

describe('measureDist：量测与产物健全性', () => {
  it('合计 gzip = 各首屏资源之和（CSS 计入首屏总量）', () => {
    const dir = track(
      makeDist({
        html: indexHtml([
          '<script type="module" src="/assets/index-abc.js"></script>',
          '<link rel="stylesheet" href="/assets/index-abc.css">',
          '<link rel="icon" href="/vite.svg">',
        ]),
        files: {
          'index-abc.js': blobOfSize(80_000),
          'index-abc.css': blobOfSize(20_000),
          'unused.js': blobOfSize(500_000),
        },
      }),
    )
    const result = measureDist(dir)
    expect(result.assets).toHaveLength(2)
    expect(result.totalRawKb).toBeCloseTo((80_000 + 20_000) / 1024, 1)
    expect(result.totalGzipKb).toBeCloseTo(
      result.assets.reduce((s, a) => s + a.gzipKb, 0),
      5,
    )
    // JS / CSS 分账：首屏总量 = 二者之和（判据是总量，分账仅用于排障定位）
    expect(result.jsGzipKb).toBeCloseTo(result.assets[0].gzipKb, 5)
    expect(result.cssGzipKb).toBeCloseTo(result.assets[1].gzipKb, 5)
  })

  it('index.html 缺失 → 明确提示先构建（避免 CI 里看不懂的 ENOENT）', () => {
    const dir = track(mkdtempSync(join(tmpdir(), 'bundle-gate-empty-')))
    expect(() => measureDist(dir)).toThrow(BundleBudgetError)
    expect(() => measureDist(dir)).toThrow(/npm run build/)
  })

  it('index.html 引用的资源不存在 → 判定产物不完整', () => {
    const dir = track(
      makeDist({
        html: indexHtml(['<script type="module" src="/assets/missing.js"></script>']),
      }),
    )
    expect(() => measureDist(dir)).toThrow(/资源不存在.*missing\.js/)
  })

  it('解析不到首屏资源 → 视为规则失配（防止「0 项通过」的假绿）', () => {
    const dir = track(makeDist({ html: indexHtml(['<link rel="icon" href="/vite.svg">']) }))
    expect(() => measureDist(dir)).toThrow(/未解析到任何首屏资源/)
  })
})

describe('run：门禁端到端（含 CI 红/绿语义）', () => {
  it('预算内：打印明细并返回量测结果，不抛错', () => {
    const dir = track(
      makeDist({
        html: indexHtml(['<script type="module" src="/assets/index-abc.js"></script>']),
        files: { 'index-abc.js': blobOfSize(200_000) },
      }),
    )
    const logs = []
    const result = run({ distDir: dir, budgetKb: 200, log: (l) => logs.push(l) })
    expect(result.ok).toBe(true)
    expect(result.assets).toHaveLength(1)
    expect(logs.join('\n')).toContain('首屏体积在预算内')
    expect(logs.join('\n')).toContain('合计 1 项')
  })

  it('超预算：抛 BundleBudgetError，含超出量、Top3 与处理建议（CI 红）', () => {
    const dir = track(
      makeDist({
        html: indexHtml([
          '<script type="module" src="/assets/a.js"></script>',
          '<link rel="modulepreload" href="/assets/b.js">',
        ]),
        files: { 'a.js': blobOfSize(900_000), 'b.js': blobOfSize(400_000) },
      }),
    )
    const logs = []
    let error
    try {
      run({ distDir: dir, budgetKb: 200, log: (l) => logs.push(l) })
    } catch (err) {
      error = err
    }
    expect(error).toBeInstanceOf(BundleBudgetError)
    expect(error.message).toContain('首屏体积超预算')
    expect(error.message).toContain('超出')
    expect(error.message).toContain('a.js')
    expect(error.message).toContain('处理建议')
    // 即使失败，报告也应已打印（排障可见）
    expect(logs.join('\n')).toContain('合计 2 项')
  })
})
