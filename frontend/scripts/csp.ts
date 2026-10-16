/**
 * 内容安全策略（CSP）单一来源（Doc/33 P-6）。
 *
 * 此前同一条策略（含首屏内联脚本的 sha256）在 `index.html`、`nginx.conf` ×3、`install.sh`
 * 五处手写，改脚本要同步五处，且 install.sh 的哈希不会随前端升级更新。现在：
 *
 * - 指令列表只在本文件声明（{@link CSP_DIRECTIVES}）；
 * - 内联脚本哈希在构建时从 `index.html` 实算，不再手写；
 * - Vite 插件把策略注入 `index.html` 的 `<meta>`（替换 {@link CSP_PLACEHOLDER}），并产出
 *   `dist/csp.conf`（nginx `add_header` 片段，追加仅头部有效的 `frame-ancestors`）；
 * - Docker `nginx.conf` 与 `install.sh` 生成的站点配置都 `include` 该片段——
 *   前端产物升级即自动携带新哈希。
 *
 * 护栏：`src/config/__tests__/csp.spec.ts`。
 */
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import type { Plugin } from 'vite'

/** meta 与 HTTP 头共用的指令（`script-src` 的哈希在构建时追加）。 */
export const CSP_DIRECTIVES: readonly string[] = [
  "default-src 'self'",
  "script-src 'self'",
  // Vue :style 绑定与 UI 库运行时注入 <style> 需要；脚本侧仍无 unsafe-inline
  "style-src 'self' 'unsafe-inline'",
  // 主题自定义背景 / 导出预览
  "img-src 'self' data: blob:",
  "font-src 'self' data:",
  // 同源 WebSocket 亦被 'self' 覆盖（现代浏览器）；跨域须显式补 wss://host，勿放宽为 *
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
]

/** 仅 HTTP 头有效的指令（meta 形式会被浏览器忽略并告警）。 */
export const HEADER_ONLY_DIRECTIVES: readonly string[] = ["frame-ancestors 'none'"]

/** `index.html` 源码中的占位符，构建 / 开发时替换为实算策略。 */
export const CSP_PLACEHOLDER = '%FLAMEPANEL_CSP%'

/** 产物中的 nginx 片段文件名（位于 dist 根）。 */
export const CSP_SNIPPET_FILE = 'csp.conf'

/** 所有无属性 `<script>…</script>` 的脚本体（即需要哈希放行的内联脚本）。 */
export function inlineScriptBodies(html: string): string[] {
  return [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1])
}

/** CSP 源表达式形式的哈希（必须带单引号，无引号形式不被浏览器识别）。 */
export function scriptHash(body: string): string {
  return `'sha256-${createHash('sha256').update(body, 'utf-8').digest('base64')}'`
}

/** 由 `index.html` 实算完整策略；`header` 为 true 时追加仅头部有效的指令。 */
export function buildPolicy(html: string, opts: { header?: boolean } = {}): string {
  const hashes = inlineScriptBodies(html).map(scriptHash)
  const directives = CSP_DIRECTIVES.map((d) =>
    d.startsWith('script-src ') && hashes.length ? `${d} ${hashes.join(' ')}` : d,
  )
  if (opts.header) directives.push(...HEADER_ONLY_DIRECTIVES)
  return directives.join('; ')
}

/** `dist/csp.conf` 内容：可在 http / server / location 任一层 `include`。 */
export function nginxSnippet(html: string): string {
  return (
    '# 由前端构建生成（frontend/scripts/csp.ts），勿手改；随前端产物升级自动更新。\n' +
    `add_header Content-Security-Policy "${buildPolicy(html, { header: true })}" always;\n`
  )
}

/** Vite 插件：注入 meta 策略 + 产出 nginx 片段。 */
export function cspPlugin(): Plugin {
  let root = process.cwd()
  return {
    name: 'flamepanel-csp',
    configResolved(config) {
      root = config.root
    },
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        if (!html.includes(CSP_PLACEHOLDER)) {
          throw new Error(`index.html 缺少 CSP 占位符 ${CSP_PLACEHOLDER}`)
        }
        return html.replace(CSP_PLACEHOLDER, buildPolicy(html))
      },
    },
    generateBundle() {
      // 直接读源 index.html：内联脚本体构建前后逐字节不变，哈希与产物一致
      const html = readFileSync(resolve(root, 'index.html'), 'utf-8')
      this.emitFile({ type: 'asset', fileName: CSP_SNIPPET_FILE, source: nginxSnippet(html) })
    },
  }
}
