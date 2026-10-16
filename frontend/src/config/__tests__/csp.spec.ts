import { describe, it, expect } from 'vitest'
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'
import {
  CSP_PLACEHOLDER,
  CSP_SNIPPET_FILE,
  HEADER_ONLY_DIRECTIVES,
  buildPolicy,
  cspPlugin,
  inlineScriptBodies,
  nginxSnippet,
  scriptHash,
} from '../../../scripts/csp'

/**
 * T0-5 内容安全策略（CSP）基线回归（Doc/23 §3.1 / 问题 P0-1、P0-5）。
 *
 * 背景：access token 存于 `localStorage`（方案 A 保留该设计），而全站没有任何 CSP——
 * 一旦出现 XSS，攻击者可内联脚本直接读走令牌。CSP 的 `script-src` 无 `unsafe-inline`
 * 是切断该链路的**唯一前端侧手段**。
 *
 * 本 spec 守住三件事：
 * 1. **白名单哈希与脚本体逐字节一致**——首屏防闪烁内联脚本是唯一被放行的内联脚本，
 *    改了脚本却忘改哈希 → 脚本被浏览器拦截（首屏闪白/闪黑），而构建与类型检查都不会报错；
 * 2. **三处下发点同源**——`index.html`（meta）/ `nginx.conf`（Docker）/ `install.sh`
 *    （systemd 裸机）必须是同一套指令，只改一处 = 生产与开发行为分叉；
 * 3. **`add_header` 继承陷阱**——nginx 中下级 `location` 一旦自带 `add_header`，
 *    上层全部头**不再继承**，CSP 会在那些路径上静默消失。
 *
 * 指令取舍的完整理由见 `frontend/index.html` 内联注释与本 spec 的断言分组。
 */

const frontendRoot = resolve(import.meta.dirname, '../../..')
const repoRoot = resolve(frontendRoot, '..')

const read = (p: string) => readFileSync(p, 'utf-8')

/** 提取首个 `<script>...</script>`（无属性）的脚本体——即 CSP 需要放行的那一个。 */
function extractInlineScripts(html: string): string[] {
  return [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1])
}

const sha256 = (body: string) =>
  'sha256-' + createHash('sha256').update(body, 'utf-8').digest('base64')

/** 从 CSP 字符串里取 `script-src` 指令的取值列表。 */
function scriptSrcOf(policy: string): string {
  return (
    policy
      .split(';')
      .map((d) => d.trim())
      .find((d) => d.startsWith('script-src')) ?? ''
  )
}

/** 取某条指令的值；不存在返回 undefined。 */
function directiveOf(policy: string, name: string): string | undefined {
  return policy
    .split(';')
    .map((d) => d.trim())
    .find((d) => d.startsWith(`${name} `))
}

/** 递归收集源码文件（跳过 node_modules / dist / 测试目录本身）。 */
function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) {
      if (['node_modules', 'dist', 'coverage', '__tests__'].includes(name)) continue
      collectSourceFiles(full, out)
    } else if (/\.(vue|ts|tsx|js|jsx|html)$/.test(name)) {
      out.push(full)
    }
  }
  return out
}

/** 剥掉注释，避免说明性文字触发断言。 */
function stripComments(text: string): string {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/.*$/gm, '$1')
    .replace(/<!--[\s\S]*?-->/g, '')
}

/**
 * 剥掉 nginx 行注释。必须先做——`#` 注释里可能出现「location」等词
 * （例如本例中解释继承规则的说明文字，以及文件末尾整块注释掉的 TLS 示例），
 * 直接正则匹配会把它们当成真实配置块。
 */
function stripNginxComments(conf: string): string {
  return conf
    .split('\n')
    .map((line) => line.replace(/(^|\s)#.*$/, ''))
    .join('\n')
}

/** 用花括号配对切出 nginx `location ... { }` 块（注释已剥离）。 */
function locationBlocks(conf: string): Array<{ header: string; body: string }> {
  const blocks: Array<{ header: string; body: string }> = []
  const re = /location\s+[^{]*\{/g
  let m: RegExpExecArray | null
  while ((m = re.exec(conf)) !== null) {
    let depth = 1
    let i = m.index + m[0].length
    while (i < conf.length && depth > 0) {
      if (conf[i] === '{') depth++
      else if (conf[i] === '}') depth--
      i++
    }
    blocks.push({
      header: m[0].replace(/\{$/, '').trim(),
      body: conf.slice(m.index + m[0].length, i - 1),
    })
  }
  return blocks
}

const indexHtml = read(join(frontendRoot, 'index.html'))
const nginxConf = read(join(repoRoot, 'nginx.conf'))
const installSh = read(join(repoRoot, 'install.sh'))

/** 构建 / 开发时注入 meta 的策略（Doc/33 P-6：由 scripts/csp.ts 实算）。 */
const metaPolicy = buildPolicy(indexHtml)
/** dist/csp.conf 片段与其中的头部策略。 */
const snippet = nginxSnippet(indexHtml)
const headerPolicy = snippet.match(/add_header\s+Content-Security-Policy\s+"([^"]*)"\s+always;/)?.[1] ?? ''
/** install.sh 旧版前端包兜底策略。 */
const installFallback = installSh.match(/CSP_FALLBACK_POLICY="([^"]*)"/)?.[1]
const DOCKER_INCLUDE = 'include /app/frontend/dist/csp.conf;'
const installIncludes = (installSh.match(/include \$CSP_SNIPPET;/g) ?? []).length

describe('T0-5 CSP 基线（P0-1 / P0-5；Doc/33 P-6 单一来源）', () => {
  describe('1. 白名单哈希由脚本体实算（不再手写）', () => {
    it('index.html 恰好保留一个无属性内联脚本，且 meta 为占位符', () => {
      const scripts = extractInlineScripts(indexHtml)
      expect(scripts.length, 'index.html 应恰好保留一个无属性内联脚本（首屏防闪烁）').toBe(1)
      expect(indexHtml).toContain(`content="${CSP_PLACEHOLDER}"`)
      // 源码中不得残留手写哈希——它会与实算值漂移
      expect(indexHtml).not.toMatch(/'sha256-[A-Za-z0-9+/=]+'/)
    })

    it('【核心回归】策略中的 sha256 与实际内联脚本体匹配', () => {
      const actual = sha256(extractInlineScripts(indexHtml)[0])
      expect(inlineScriptBodies(indexHtml)).toEqual(extractInlineScripts(indexHtml))
      expect(scriptHash(extractInlineScripts(indexHtml)[0])).toBe(`'${actual}'`)
      expect(metaPolicy).toContain(`'${actual}'`)
      expect(headerPolicy).toContain(`'${actual}'`)
    })

    it('【核心回归】哈希必须带单引号——未加引号的 CSP hash 不被浏览器识别', () => {
      // 实测：Chrome 对 `script-src 'self' sha256-xxx`（无引号）一律拦截并报
      // 「Either the unsafe-inline keyword, a hash ... is required」。写成无引号
      // 时**构建、类型检查、lint 全部通过**，只有真实浏览器会静默拦掉首屏脚本。
      const bare = sha256(extractInlineScripts(indexHtml)[0])
      expect(metaPolicy).toContain(`'${bare}'`)
      expect(metaPolicy).not.toMatch(new RegExp(`[^']${bare.replace(/[+/]/g, '\\$&')}`))
    })

    it('meta 出现在内联脚本之前（meta CSP 只能约束其后的内容）', () => {
      expect(indexHtml.indexOf('Content-Security-Policy')).toBeLessThan(
        indexHtml.indexOf('<script>'),
      )
    })

    it('Vite 插件替换占位符为实算策略，缺占位符时构建失败', () => {
      const plugin = cspPlugin()
      const hook = plugin.transformIndexHtml as { handler: (html: string) => string }
      const out = hook.handler(indexHtml)
      expect(out).not.toContain(CSP_PLACEHOLDER)
      expect(out).toContain(`content="${metaPolicy}"`)
      expect(() => hook.handler('<html></html>')).toThrow(CSP_PLACEHOLDER)
    })
  })

  describe('2. 指令面：禁内联脚本执行、封死高危向量', () => {
    it('【核心回归】script-src 不含 unsafe-inline / unsafe-eval', () => {
      const src = scriptSrcOf(metaPolicy)
      expect(src).not.toContain('unsafe-inline')
      expect(src).not.toContain('unsafe-eval')
      expect(src).toContain("'self'")
    })

    it('封死 object / base / form 三个注入逃逸面', () => {
      expect(directiveOf(metaPolicy, 'object-src')).toBe("object-src 'none'")
      expect(directiveOf(metaPolicy, 'base-uri')).toBe("base-uri 'self'")
      expect(directiveOf(metaPolicy, 'form-action')).toBe("form-action 'self'")
      expect(directiveOf(metaPolicy, 'default-src')).toBe("default-src 'self'")
    })

    it('style-src 保留 unsafe-inline（Vue :style 与 UI 库运行时注入 style 需要）', () => {
      expect(directiveOf(metaPolicy, 'style-src')).toContain("'unsafe-inline'")
    })

    it('img-src 覆盖 data: 与 blob:（主题自定义背景 / 导出预览）', () => {
      const img = directiveOf(metaPolicy, 'img-src')!
      expect(img).toContain('data:')
      expect(img).toContain('blob:')
      expect(directiveOf(metaPolicy, 'font-src')).toContain('data:')
    })

    it('connect-src 为 self——同源 WebSocket 亦被覆盖（实测验证）', () => {
      expect(directiveOf(metaPolicy, 'connect-src')).toBe("connect-src 'self'")
      expect(metaPolicy).not.toContain('connect-src *')
    })
  })

  describe('3. 下发点同源：meta / dist/csp.conf / install.sh 兜底', () => {
    it('头部策略 = meta 策略 + frame-ancestors（仅头部有效）', () => {
      expect(headerPolicy.startsWith(metaPolicy)).toBe(true)
      expect(headerPolicy.slice(metaPolicy.length)).toBe(
        `; ${HEADER_ONLY_DIRECTIVES.join('; ')}`,
      )
      expect(snippet).toMatch(/^# /)
      expect(CSP_SNIPPET_FILE).toBe('csp.conf')
    })

    it('frame-ancestors 只在 HTTP 头出现——meta 形式会被浏览器忽略', () => {
      expect(metaPolicy).not.toContain('frame-ancestors')
      expect(headerPolicy).toContain("frame-ancestors 'none'")
    })

    it('nginx.conf / install.sh 不再内联策略，只 include 构建产物', () => {
      expect(nginxConf).not.toMatch(/add_header\s+Content-Security-Policy/)
      expect(nginxConf).not.toContain('sha256-')
      expect(installSh).not.toContain('sha256-')
      expect(installSh).not.toMatch(/add_header\s+Content-Security-Policy\s+"\$CSP_POLICY"/)
    })

    it('install.sh 兜底策略 = 头部策略去掉内联脚本哈希（指令不分叉）', () => {
      const withoutHash = headerPolicy.replace(/ '(sha256-[A-Za-z0-9+/=]+)'/g, '')
      expect(installFallback).toBe(withoutHash)
    })
  })

  describe('4. add_header 继承陷阱：自带头的块必须重复 include', () => {
    it('【核心回归】nginx.conf 中每个自带 add_header 的 location 都 include CSP 片段', () => {
      const offenders = locationBlocks(stripNginxComments(nginxConf))
        .filter((b) => b.body.includes('add_header') && !b.body.includes(DOCKER_INCLUDE))
        .map((b) => b.header)
      expect(offenders).toEqual([])
      // http 级 + 两个自带头的 location
      expect(nginxConf.split(DOCKER_INCLUDE).length - 1).toBeGreaterThanOrEqual(3)
    })

    it('install.sh 生成的每个 server 与自带头的 location 均 include 片段', () => {
      // 80（TLS 重定向）/ 主站 server + /index.html + /assets/
      expect(installIncludes).toBeGreaterThanOrEqual(4)
      expect(installSh).toContain('CSP_SNIPPET="$FRONTEND_DIR/dist/csp.conf"')
    })
  })

  describe('5. 源码侧护栏：不引入会被 CSP 静默拦掉或需要 eval 的写法', () => {
    it('无内联事件属性（onclick=/onerror=…）与 javascript: URL', () => {
      const offenders: string[] = []
      for (const file of collectSourceFiles(join(frontendRoot, 'src'))) {
        const text = stripComments(read(file))
        if (/\son(click|error|load|change|input|submit|focus|blur)\s*=\s*["']/.test(text)) {
          offenders.push(`${file.replace(`${frontendRoot}/`, '')}: 内联事件属性`)
        }
        if (/javascript:/i.test(text)) {
          offenders.push(`${file.replace(`${frontendRoot}/`, '')}: javascript: URL`)
        }
      }
      expect(offenders).toEqual([])
    })

    it('无 new Function / eval（unsafe-eval 未放行，用了即运行时崩溃）', () => {
      const offenders: string[] = []
      for (const file of collectSourceFiles(join(frontendRoot, 'src'))) {
        const text = stripComments(read(file))
        if (/\bnew Function\b|\beval\s*\(/.test(text)) {
          offenders.push(file.replace(`${frontendRoot}/`, ''))
        }
      }
      expect(offenders).toEqual([])
    })

    it('index.html 保留的 CSP 说明注释（改策略前先读理由）', () => {
      expect(indexHtml).toContain('T0-5')
      expect(indexHtml).toContain('scripts/csp.ts')
    })
  })
})
