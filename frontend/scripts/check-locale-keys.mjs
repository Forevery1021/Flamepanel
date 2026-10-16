#!/usr/bin/env node
/**
 * T2-5 i18n key 一致性门禁（Doc/23 §3.3 T2-5，问题 P1-6「单文件巨型化」/ R-6「裸 key 上屏」）。
 *
 * 为什么需要它：语言包从「三个千行文件」拆为「按业务域一语言一目录」之后，
 * 两种事故从「改错一行」升级为「结构性静默退化」——
 * 1. 某语言模块漏了一份（切到该语言时整块文案变裸 key）；
 * 2. 代码新增 `t('foo.bar')` 但三语言都没补（界面上直接显示 `foo.bar`，R-6 的形态）。
 * 二者都不会让 typecheck / lint / 既有单测变红（裸 key 是合法字符串），故独立成门禁。
 *
 * 四类校验（全部为「机器可读」判据）：
 * A. 结构一致：三个语言目录的模块清单（`index.ts` 的 import 列表）必须完全一致；
 * B. key 集合一致：三语言扁平化后的 key 集合必须完全一致（哪个语言缺哪条精确报出）；
 * C. 占位符一致：同一 key 三语言的 `{param}` 集合一致（防某语言漏译参数导致句子残缺）；
 * D. 引用存在：`src/**` 里 `t('a.b')` 之类的静态 key 字面量必须在三语言里都存在；
 *    动态拼接（`t('a.b_' + x)`、`` t(`a.b${x}`) ``）按前缀校验「至少有一条 key 命中」；
 *    路由 `meta.title: 'nav.x'` 同为 i18n key（侧栏/页签/⌘K 三处消费），一并纳入。
 *    刻意不做「全量 key 形状字面量」扫描：`app.config.errorHandler`、`app.id` 这类
 *    非 i18n 字面量会产生误报，把门禁噪音当红线用，最终只会被绕过。
 *
 * 为何用 typescript 而非正则解析语言包：本仓已依赖 typescript（devDependency），
 * 解析走 AST 可精确覆盖多行/引号/嵌套，不引入新依赖，也不受排版变化影响。
 *
 * 用法：
 *   npm run locale:check                                   # 默认校验 src/locales 与 src
 *   node scripts/check-locale-keys.mjs --src-dir=./tmp/src # 排障：换扫描根
 * 退出码：0 全绿；1 有差异（CI 红 / lint-staged 拦截提交）。
 */
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import ts from 'typescript'

/** 语言目录固定三语（新增语种时在此登记，并同步 src/locales/index.ts） */
export const LOCALE_LANGS = ['zh-CN', 'en-US', 'ja-JP']

/** 默认扫描根（相对 frontend/） */
export const DEFAULT_LOCALES_DIR = 'src/locales'
export const DEFAULT_SRC_DIR = 'src'

/** 引用扫描：跳过的目录 / 文件（生成物、静态文案自身、测试夹具） */
const SKIP_DIRS = new Set(['node_modules', 'dist', 'coverage', 'locales', 'generated', '__tests__'])
const SKIP_FILE_RE = /\.(spec|test|d)\.ts$/

export class LocaleKeysError extends Error {
  constructor(message, problems = []) {
    super(message)
    this.name = 'LocaleKeysError'
    this.problems = problems
  }
}

/** 读取语言模块文件，返回 { 命名空间: 文案树 }（AST 解析，非字符串值直接报错） */
export function readPackFile(filePath) {
  const source = readFileSync(filePath, 'utf8')
  const sf = ts.createSourceFile(filePath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  let expr
  for (const st of sf.statements) {
    if (ts.isExportAssignment(st) && !st.isExportEquals) expr = st.expression
  }
  if (!expr) throw new Error(`${filePath}: 缺少 export default`)
  const convert = (node, path) => {
    if (ts.isObjectLiteralExpression(node)) {
      const out = {}
      for (const prop of node.properties) {
        if (!ts.isPropertyAssignment(prop)) {
          throw new Error(
            `${filePath}: ${path} 处不是 key: value 形式（i18n 语言包只允许字面量对象）`,
          )
        }
        const name = prop.name.getText(sf).replace(/^['"`]|['"`]$/g, '')
        out[name] = convert(prop.initializer, path ? `${path}.${name}` : name)
      }
      return out
    }
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text
    throw new Error(
      `${filePath}: ${path} 的值不是字符串字面量（模板变量/表达式会让语言包无法静态校验）`,
    )
  }
  return convert(expr, '')
}

/** 解析某语言的 index.ts，返回有序模块清单（聚合顺序 = 文件顺序） */
export function readModuleManifest(indexFile) {
  const source = readFileSync(indexFile, 'utf8')
  const modules = []
  for (const line of source.split('\n')) {
    const m = /^import\s+\w+\s+from\s+'\.\/([\w-]+)'\s*$/.exec(line.trim())
    if (m) modules.push(m[1])
  }
  return modules
}

/** 扁平化：嵌套对象 → Map<'a.b.c', string>；重名命名空间/键在此处即冲突报错 */
export function flattenPack(tree, { onConflict } = {}) {
  const flat = new Map()
  const walk = (node, prefix) => {
    for (const [key, value] of Object.entries(node)) {
      const path = prefix ? `${prefix}.${key}` : key
      if (value !== null && typeof value === 'object') {
        walk(value, path)
      } else {
        if (flat.has(path) && onConflict) onConflict(path)
        flat.set(path, value)
      }
    }
  }
  walk(tree, '')
  return flat
}

/**
 * 提取 `{param}` 占位符集合（去重后排序，便于直接比较）。
 *
 * 去重而非计数：同一参数在句中出现两次（中英句式差异）不影响渲染正确性，
 * 把它算成差异只会制造假红——门禁要拦的是「某语言整段漏了参数」。
 */
export function placeholders(text) {
  return [...new Set([...String(text).matchAll(/\{(\w+)\}/g)].map((m) => m[1]))].sort()
}

/** 从单个源码文件里抽取 i18n key 引用（静态 key + 动态前缀，含行号） */
export function scanSource(filePath, source) {
  const statics = []
  const prefixes = []
  const lineOf = (index) => source.slice(0, index).split('\n').length
  // 形态一：t('a.b') / te('a.b') / tm(...) / tt('a.b', fallback)；排除 obj.t(...) 方法调用
  const literalRe = /(?<![\w$.$])(?:t|te|tm|tt)\(\s*(['"])([A-Za-z][\w$]*(?:\.[\w$]+)*)\1/g
  for (const m of source.matchAll(literalRe)) {
    const key = m[2]
    const trailing = source[m.index + m[0].length]
    if (key.endsWith('_') || key.endsWith('.') || trailing === '.' || trailing === '+') {
      prefixes.push({ key, file: filePath, line: lineOf(m.index) })
    } else {
      statics.push({ key, file: filePath, line: lineOf(m.index) })
    }
  }
  // 形态二：t(`a.b${x}`) —— 模板拼接，取 ${ 前的字面前缀
  const templateRe = /(?<![\w$.$])(?:t|te|tm|tt)\(\s*`([^`]*?)\$\{/g
  for (const m of source.matchAll(templateRe)) {
    if (m[1] && /^[A-Za-z][\w$]*(\.[\w$]*)*$/.test(m[1])) {
      prefixes.push({ key: m[1], file: filePath, line: lineOf(m.index) })
    }
  }
  // 形态三：路由 meta.title: 'a.b' —— 侧栏 / 页签 / ⌘K 三处都经 t(meta.title) 取值
  // （Doc/23 §7.18 的待决点：nav.* 是否纳入校验；此处纳入，故 `nav.appStore`
  //   这类「菜单裸 key」从此被机器拦下，不再依赖真实浏览器复验才发现）
  const metaTitleRe = /\btitle:\s*(['"])([A-Za-z][\w$]*(?:\.[\w$]+)+)\1/g
  for (const m of source.matchAll(metaTitleRe)) {
    statics.push({ key: m[2], file: filePath, line: lineOf(m.index) })
  }
  return { statics, prefixes }
}

/** 递归收集待扫描源码文件（.ts / .vue，跳过生成物与测试） */
export function collectSourceFiles(root) {
  const files = []
  const walk = (dir) => {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry)
      const stat = statSync(full)
      if (stat.isDirectory()) {
        if (!SKIP_DIRS.has(entry)) walk(full)
        continue
      }
      if (!/\.(ts|vue)$/.test(entry) || SKIP_FILE_RE.test(entry)) continue
      files.push(full)
    }
  }
  if (statSync(root).isDirectory()) walk(root)
  return files.sort()
}

/** 纯校验入口：返回 { ok, problems, stats }，不做 IO 之外的副作用（可直接单测） */
export function checkLocaleKeys({ localesDir, srcDir }) {
  const problems = []
  const stats = { langs: [], keysPerLang: {}, modules: 0, refs: 0, prefixes: 0 }

  // —— A. 结构一致：三个语言的模块清单必须完全相同
  const manifests = {}
  for (const lang of LOCALE_LANGS) {
    const indexFile = join(localesDir, lang, 'index.ts')
    let modules
    try {
      modules = readModuleManifest(indexFile)
    } catch {
      problems.push(`语言 ${lang} 缺少聚合入口 ${relative(process.cwd(), indexFile)}`)
      manifests[lang] = []
      continue
    }
    if (modules.length === 0) problems.push(`语言 ${lang} 的 index.ts 未聚合任何模块`)
    manifests[lang] = modules
  }
  const reference = LOCALE_LANGS[0]
  for (const lang of LOCALE_LANGS.slice(1)) {
    const missing = manifests[reference].filter((m) => !manifests[lang].includes(m))
    const extra = manifests[lang].filter((m) => !manifests[reference].includes(m))
    if (missing.length) problems.push(`语言 ${lang} 缺少模块：${missing.join(', ')}`)
    if (extra.length) problems.push(`语言 ${lang} 多出模块：${extra.join(', ')}`)
    if (
      !missing.length &&
      !extra.length &&
      manifests[lang].join() !== manifests[reference].join()
    ) {
      problems.push(`语言 ${lang} 的模块聚合顺序与 ${reference} 不一致（index.ts 建议同序维护）`)
    }
  }
  stats.modules = manifests[reference].length

  // —— B/C. 逐语言装载并扁平化：key 集合一致 + 占位符一致 + 无空值
  const packs = {}
  for (const lang of LOCALE_LANGS) {
    const merged = {}
    const owners = new Map() // key → 模块文件（重名即两份模块写同一命名空间）
    for (const mod of manifests[lang]) {
      const file = join(localesDir, lang, `${mod}.ts`)
      let tree
      try {
        tree = readPackFile(file)
      } catch (err) {
        problems.push(`${lang}/${mod}.ts 解析失败：${err.message}`)
        continue
      }
      for (const ns of Object.keys(tree)) {
        if (ns in merged) {
          problems.push(
            `${lang}/${mod}.ts 与 ${owners.get(ns)} 重复定义命名空间「${ns}」（key 会被静默覆盖）`,
          )
        } else {
          owners.set(ns, `${mod}.ts`)
        }
      }
      Object.assign(merged, tree)
    }
    const flat = flattenPack(merged, {
      onConflict: (key) => problems.push(`语言 ${lang} 内 key「${key}」重复定义（后写覆盖前写）`),
    })
    for (const [key, value] of flat) {
      if (!String(value).trim())
        problems.push(`语言 ${lang} 的 key「${key}」为空串（界面上会渲染空白）`)
    }
    packs[lang] = flat
    stats.langs.push(lang)
    stats.keysPerLang[lang] = flat.size
  }

  // key 集合差异：以首个语言为基准，逐条报出缺/多
  const baseKeys = packs[reference] ?? new Map()
  const allKeys = new Set(Object.values(packs).flatMap((pack) => [...pack.keys()]))
  for (const key of [...allKeys].sort()) {
    for (const lang of LOCALE_LANGS) {
      if (!packs[lang]?.has(key))
        problems.push(`key「${key}」在 ${lang} 缺失（基准 ${reference} 有）`)
    }
  }
  for (const key of [...baseKeys.keys()].sort()) {
    const tokens = placeholders(baseKeys.get(key))
    for (const lang of LOCALE_LANGS) {
      const value = packs[lang]?.get(key)
      if (value === undefined) continue
      const other = placeholders(value)
      if (tokens.join() !== other.join()) {
        problems.push(
          `key「${key}」占位符不一致：${reference}=[${tokens}] ${lang}=[${other}]（参数会原样透出或丢失）`,
        )
      }
    }
  }

  // —— D. 引用存在：源码里静态 key 必须在三语言都存在；动态前缀至少有一条 key 命中
  const sourceFiles = collectSourceFiles(srcDir)
  const staticRefs = new Map()
  const prefixRefs = new Map()
  for (const file of sourceFiles) {
    const { statics, prefixes } = scanSource(file, readFileSync(file, 'utf8'))
    for (const ref of statics) {
      const hit = staticRefs.get(ref.key)
      if (hit) hit.hits.push(ref)
      else staticRefs.set(ref.key, { key: ref.key, hits: [ref] })
    }
    for (const ref of prefixes) {
      const hit = prefixRefs.get(ref.key)
      if (hit) hit.hits.push(ref)
      else prefixRefs.set(ref.key, { key: ref.key, hits: [ref] })
    }
  }
  stats.refs = staticRefs.size
  stats.prefixes = prefixRefs.size
  for (const { key, hits } of staticRefs.values()) {
    for (const lang of LOCALE_LANGS) {
      if (!packs[lang]?.has(key)) {
        const where = hits.slice(0, 3).map((h) => `${relative(process.cwd(), h.file)}:${h.line}`)
        const more = hits.length > 3 ? ` 等 ${hits.length} 处` : ''
        problems.push(
          `源码引用 @${where.join(' ')}${more} 的 key「${key}」在 ${lang} 不存在（界面会显示裸 key）`,
        )
      }
    }
  }
  for (const { key, hits } of prefixRefs.values()) {
    for (const lang of LOCALE_LANGS) {
      const pack = packs[lang]
      if (!pack) continue
      const matched = [...pack.keys()].some((k) => k.startsWith(key))
      if (!matched) {
        const where = hits.slice(0, 3).map((h) => `${relative(process.cwd(), h.file)}:${h.line}`)
        problems.push(
          `动态前缀「${key}」@${where.join(' ')} 在 ${lang} 下没有任何 key 命中（拼接结果必为裸 key）`,
        )
      }
    }
  }

  return { ok: problems.length === 0, problems, stats }
}

/** CLI 入口：打印报告，失败抛 LocaleKeysError（供测试断言与 CI 退出码） */
export function run({
  localesDir = DEFAULT_LOCALES_DIR,
  srcDir = DEFAULT_SRC_DIR,
  log = console.log,
} = {}) {
  const result = checkLocaleKeys({ localesDir, srcDir })
  const { stats } = result
  log(`语言模块：${stats.modules} 个模块 × ${stats.langs.length} 语言`)
  log(`语言包：${stats.langs.map((l) => `${l} ${stats.keysPerLang[l]} key`).join(' · ')}`)
  log(`源码引用：静态 key ${stats.refs} 个 · 动态前缀 ${stats.prefixes} 个`)
  if (!result.ok) {
    log(`\n发现 ${result.problems.length} 处问题：`)
    for (const p of result.problems) log(`  ✗ ${p}`)
    throw new LocaleKeysError(
      `i18n key 一致性校验未通过（${result.problems.length} 处）`,
      result.problems,
    )
  }
  log('i18n key 一致性校验通过：三语言 key 集合一致、引用全部命中。')
  return result
}

/** 轻量参数解析：只认 --k=v，其余（lint-staged 追加的暂存文件列表）忽略 */
export function parseArgs(argv) {
  const opts = {}
  for (const arg of argv) {
    const m = /^--([\w-]+)=(.*)$/.exec(arg)
    if (!m) continue
    if (m[1] === 'locales-dir') opts.localesDir = m[2]
    if (m[1] === 'src-dir') opts.srcDir = m[2]
  }
  return opts
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href
if (invokedDirectly) {
  try {
    run(parseArgs(process.argv.slice(2)))
  } catch (err) {
    if (err instanceof LocaleKeysError) process.exit(1)
    throw err
  }
}
